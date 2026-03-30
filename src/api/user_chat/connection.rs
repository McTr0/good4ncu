use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token_with_fallback;
use crate::api::error::ApiError;
use crate::api::ws;
use crate::api::AppState;

use super::{
    ConnectAcceptBody, ConnectAcceptResponse, ConnectRejectBody, ConnectRejectResponse,
    ConnectRequestBody, ConnectRequestResponse, ConnectionEntry, ConnectionListResponse,
    WsConnectionEstablishedEvent, WsConnectionRejectedEvent, WsConnectionRequestEvent,
};

/// POST /api/chat/connect/request — initiate a connection request (step 1 of 3-way handshake).
///
/// Creates a pending connection record and pushes a `connection_request` WS event to the receiver.
pub async fn connect_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ConnectRequestBody>,
) -> Result<Json<ConnectRequestResponse>, ApiError> {
    let requester_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    if requester_id == body.receiver_id {
        return Err(ApiError::BadRequest("不能向自己发起连接".to_string()));
    }

    let receiver_exists = sqlx::query("SELECT 1 FROM users WHERE id = $1")
        .bind(&body.receiver_id)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .is_some();
    if !receiver_exists {
        return Err(ApiError::NotFound);
    }

    let connection_id: String = {
        let existing = sqlx::query(
            "SELECT id, status FROM chat_connections 
             WHERE LEAST(requester_id, receiver_id) = LEAST($1, $2)
               AND GREATEST(requester_id, receiver_id) = GREATEST($1, $2)",
        )
        .bind(&requester_id)
        .bind(&body.receiver_id)
        .fetch_optional(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        match existing {
            Some(row) => {
                let id: uuid::Uuid = row
                    .try_get("id")
                    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
                let status: String = row.get("status");

                if status != "connected" {
                    sqlx::query(
                        "UPDATE chat_connections SET status = 'pending', requester_id = $1, receiver_id = $2, established_at = NULL WHERE id = $3"
                    )
                    .bind(&requester_id)
                    .bind(&body.receiver_id)
                    .bind(id)
                    .execute(&state.infra.db)
                    .await
                    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
                }
                id.to_string()
            }
            None => {
                let row = sqlx::query(
                    r#"INSERT INTO chat_connections (requester_id, receiver_id, status)
                       VALUES ($1, $2, 'pending')
                       RETURNING id"#,
                )
                .bind(&requester_id)
                .bind(&body.receiver_id)
                .fetch_one(&state.infra.db)
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

                let uuid_val: uuid::Uuid = row
                    .try_get("id")
                    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
                uuid_val.to_string()
            }
        }
    };

    let requester_username: Option<String> =
        sqlx::query("SELECT username FROM users WHERE id = $1")
            .bind(&requester_id)
            .fetch_optional(&state.infra.db)
            .await
            .ok()
            .flatten()
            .map(|row| row.get("username"));

    let ws_event = WsConnectionRequestEvent {
        event: "connection_request".to_string(),
        connection_id: connection_id.clone(),
        requester_id: requester_id.clone(),
        requester_username,
        listing_id: body.listing_id,
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();
    ws::broadcast_to_user(&body.receiver_id, &payload);

    Ok(Json(ConnectRequestResponse {
        connection_id,
        status: "pending".to_string(),
    }))
}

/// POST /api/chat/connect/accept — accept a connection request (step 2 of handshake).
///
/// Updates status to 'connected' and pushes `connection_established` to both parties.
pub async fn connect_accept(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ConnectAcceptBody>,
) -> Result<Json<ConnectAcceptResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    tracing::info!(user_id = %user_id, connection_id = %body.connection_id, "ACCEPT_CONNECTION");

    let row = sqlx::query(
        "SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1::uuid",
    )
    .bind(&body.connection_id)
    .fetch_optional(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let requester_id: String = row.get("requester_id");
    let receiver_id: String = row.get("receiver_id");
    let current_status: String = row.get("status");

    tracing::info!(receiver_id = %receiver_id, requester_id = %requester_id, current_status = %current_status, "ACCEPT_CONNECTION row found");

    if receiver_id != user_id {
        tracing::warn!(user_id = %user_id, receiver_id = %receiver_id, "ACCEPT_CONNECTION forbidden - not receiver");
        return Err(ApiError::Forbidden);
    }
    if current_status != "pending" {
        tracing::warn!(connection_id = %body.connection_id, current_status = %current_status, "ACCEPT_CONNECTION bad request - not pending");
        return Err(ApiError::BadRequest(format!(
            "连接状态不是 pending，当前状态: {}",
            current_status
        )));
    }

    let established_at = chrono::Utc::now();
    let established_at_str = established_at.to_rfc3339();

    sqlx::query(
        "UPDATE chat_connections SET status = 'connected', established_at = $1 WHERE id = $2::uuid",
    )
    .bind(established_at)
    .bind(&body.connection_id)
    .execute(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let ws_event = WsConnectionEstablishedEvent {
        event: "connection_established".to_string(),
        connection_id: body.connection_id.clone(),
        established_at: established_at_str.clone(),
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();
    ws::broadcast_to_user(&requester_id, &payload);
    ws::broadcast_to_user(&receiver_id, &payload);

    Ok(Json(ConnectAcceptResponse {
        status: "connected".to_string(),
        established_at: established_at_str,
    }))
}

/// POST /api/chat/connect/reject — reject a connection request.
pub async fn connect_reject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ConnectRejectBody>,
) -> Result<Json<ConnectRejectResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let row = sqlx::query(
        "SELECT requester_id, receiver_id, status FROM chat_connections WHERE id = $1::uuid",
    )
    .bind(&body.connection_id)
    .fetch_optional(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let requester_id: String = row.get("requester_id");
    let receiver_id: String = row.get("receiver_id");
    let current_status: String = row.get("status");

    if receiver_id != user_id {
        return Err(ApiError::Forbidden);
    }
    if current_status != "pending" {
        return Err(ApiError::BadRequest(format!(
            "连接状态不是 pending，当前状态: {}",
            current_status
        )));
    }

    sqlx::query("UPDATE chat_connections SET status = 'rejected' WHERE id = $1::uuid")
        .bind(&body.connection_id)
        .execute(&state.infra.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Notify the requester so they know the invitation was rejected.
    let ws_event = WsConnectionRejectedEvent {
        event: "connection_rejected".to_string(),
        connection_id: body.connection_id.clone(),
    };
    let payload = serde_json::to_string(&ws_event).unwrap_or_default();
    ws::broadcast_to_user(&requester_id, &payload);

    Ok(Json(ConnectRejectResponse {
        status: "rejected".to_string(),
    }))
}

/// GET /api/chat/connections — list all connections for the current user.
pub async fn list_connections(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ConnectionListResponse>, ApiError> {
    let user_id = extract_user_id_from_token_with_fallback(
        &headers,
        &state.secrets.jwt_secret,
        state.secrets.jwt_secret_old.as_deref(),
    )
    .map_err(|_| ApiError::Unauthorized)?;

    let rows = sqlx::query(
        r#"SELECT
               cc.id,
               cc.status,
               cc.established_at,
               cc.created_at,
               cc.unread_count,
               cc.requester_id,
               (cc.receiver_id = $1) as is_receiver,
               CASE WHEN cc.requester_id = $1 THEN cc.receiver_id ELSE cc.requester_id END as other_user_id
           FROM chat_connections cc
           WHERE cc.requester_id = $1 OR cc.receiver_id = $1
           ORDER BY cc.created_at DESC"#,
    )
    .bind(&user_id)
    .fetch_all(&state.infra.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let other_ids: Vec<String> = rows
        .iter()
        .map(|row| row.get::<String, _>("other_user_id"))
        .collect();
    let usernames: std::collections::HashMap<String, String> = if other_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        sqlx::query("SELECT id, username FROM users WHERE id = ANY($1)")
            .bind(&other_ids)
            .fetch_all(&state.infra.db)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
            .into_iter()
            .map(|row| (row.get::<String, _>("id"), row.get::<String, _>("username")))
            .collect()
    };

    let items: Vec<ConnectionEntry> = rows
        .into_iter()
        .map(|row| {
            let other_user_id: String = row.get("other_user_id");
            let established_at: Option<chrono::DateTime<chrono::Utc>> =
                row.try_get("established_at").ok().flatten();
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let unread_count: i32 = row.try_get("unread_count").unwrap_or(0);
            ConnectionEntry {
                id: row
                    .try_get::<uuid::Uuid, _>("id")
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
                requester_id: row.get("requester_id"),
                other_user_id: other_user_id.clone(),
                other_username: usernames.get(&other_user_id).cloned(),
                status: row.get("status"),
                established_at: established_at.map(|dt| dt.to_rfc3339()),
                created_at: created_at.to_rfc3339(),
                unread_count,
                is_receiver: row.get("is_receiver"),
            }
        })
        .collect();

    Ok(Json(ConnectionListResponse { items }))
}
