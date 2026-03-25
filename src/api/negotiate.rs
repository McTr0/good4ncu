//! Negotiation HITL API — seller approval via REST instead of CLI channel.
//!
//! Flow: marketplace agent calls NegotiationItemTool → creates pending HITL request
//! → seller gets notified (via /api/notifications) → responds via PATCH /api/negotiations/{id}
//! → HITL request resolved → notification sent to buyer.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::api::auth::extract_user_id_from_token;
use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Serialize)]
pub struct HitlRequestItem {
    pub id: String,
    pub listing_id: String,
    pub buyer_id: String,
    pub seller_id: String,
    pub proposed_price: f64,
    pub reason: String,
    pub status: String,
    pub counter_price: Option<f64>,
    pub created_at: String,
}

/// GET /api/negotiations — list the current user's pending negotiation requests
/// (for sellers: requests awaiting their approval; for buyers: their sent offers)
pub async fn list_negotiations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(_params): Json<ListNegotiationsParams>,
) -> Result<Json<ListNegotiationsResponse>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    let rows = sqlx::query(
        r#"
        SELECT id, listing_id, buyer_id, seller_id, proposed_price, reason, status,
               counter_price, created_at
        FROM hitl_requests
        WHERE seller_id = $1 AND status = 'pending'
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .bind(&user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let items: Vec<HitlRequestItem> = rows
        .iter()
        .map(|row| HitlRequestItem {
            id: row.get("id"),
            listing_id: row.get("listing_id"),
            buyer_id: row.get("buyer_id"),
            seller_id: row.get("seller_id"),
            proposed_price: crate::utils::cents_to_yuan(row.get::<i32, _>("proposed_price") as i64),
            reason: row.get("reason"),
            status: row.get("status"),
            counter_price: row
                .try_get::<Option<i32>, _>("counter_price")
                .ok()
                .flatten()
                .map(|c| crate::utils::cents_to_yuan(c as i64)),
            created_at: row
                .try_get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|_| String::new()),
        })
        .collect();

    Ok(Json(ListNegotiationsResponse { items }))
}

#[derive(Deserialize)]
pub struct ListNegotiationsParams {}

#[derive(Serialize)]
pub struct ListNegotiationsResponse {
    pub items: Vec<HitlRequestItem>,
}

/// PATCH /api/negotiations/{id}/respond — seller responds to a pending negotiation request
///
/// body: { "action": "approve" | "reject" | "counter", "counter_price": 180000 }
pub async fn respond_negotiation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<NegotiationResponse>,
) -> Result<Json<NegotiationResponseResult>, ApiError> {
    let user_id = extract_user_id_from_token(&headers, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized)?;

    // Fetch the request and verify ownership
    let row = sqlx::query(
        "SELECT id, seller_id, listing_id, buyer_id, status FROM hitl_requests WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?
    .ok_or(ApiError::NotFound)?;

    let owner_id: String = row.get("seller_id");
    if owner_id != user_id {
        return Err(ApiError::Forbidden);
    }

    let current_status: String = row.get("status");
    if current_status != "pending" {
        return Err(ApiError::BadRequest(
            "该议价请求已处理".to_string(),
        ));
    }

    let listing_id: String = row.get("listing_id");
    let buyer_id: String = row.get("buyer_id");

    let (new_status, counter_price) = match payload.action.as_str() {
        "approve" => ("approved", None),
        "reject" => ("rejected", None),
        "counter" => {
            let cp = payload.counter_price.ok_or_else(|| {
                ApiError::BadRequest("counter 操作需要提供 counter_price".to_string())
            })?;
            ("countered", Some(cp))
        }
        _ => {
            return Err(ApiError::BadRequest(
                "action 必须是 approve/reject/counter".to_string(),
            ))
        }
    };

    sqlx::query(
        r#"UPDATE hitl_requests
           SET status = $1, counter_price = $2, resolved_at = CURRENT_TIMESTAMP
           WHERE id = $3"#,
    )
    .bind(new_status)
    .bind(counter_price)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Fetch proposed_price before we emit DealReached (needed for system message + event)
    let hitl_row = sqlx::query("SELECT proposed_price FROM hitl_requests WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("DB error: {}", e)))?;
    let proposed_price: i64 = hitl_row.get::<i32, _>("proposed_price") as i64;

    // Notify the buyer about the seller's decision
    let (notif_title, notif_body): (String, String) = match new_status {
        "approved" => (
            "卖家接受了您的还价".to_string(),
            "卖家接受了您的还价，商品即将为您创建订单".to_string(),
        ),
        "rejected" => (
            "卖家拒绝了您的还价".to_string(),
            "抱歉，卖家未能接受您的还价".to_string(),
        ),
        "countered" => (
            "卖家还价了".to_string(),
            format!("卖家提出还价 ¥{:.2}", crate::utils::cents_to_yuan(counter_price.unwrap())),
        ),
        _ => unreachable!(),
    };

    let _ = state
        .notification
        .create(
            &buyer_id,
            "negotiation_response",
            &notif_title,
            &notif_body,
            Some(&id),
            Some(&listing_id),
        )
        .await;

    // Inject a system message into the conversation so the buyer sees it in chat history.
    // Uses sender='system' as a convention; the Flutter UI renders system messages distinctly.
    let (system_content, final_price_for_deal): (String, Option<i64>) =
        match new_status {
            "approved" => {
                let price = proposed_price;
                (
                    format!(
                        "系统：卖家接受了您的还价 ¥{:.2}，订单已创建",
                        crate::utils::cents_to_yuan(price)
                    ),
                    Some(price),
                )
            }
            "rejected" => (
                "系统：卖家拒绝了您的还价，交易取消".to_string(),
                None,
            ),
            "countered" => (
                format!(
                    "系统：卖家还价 ¥{:.2}",
                    crate::utils::cents_to_yuan(counter_price.unwrap())
                ),
                None,
            ),
            _ => unreachable!(),
        };
    let conversation_id = format!("negotiate:{}", listing_id);
    let _ = sqlx::query(
        r#"INSERT INTO chat_messages (conversation_id, sender, is_agent, content, listing_id)
           VALUES ($1, 'system', FALSE, $2, $3)"#,
    )
    .bind(&conversation_id)
    .bind(&system_content)
    .bind(&listing_id)
    .execute(&state.db)
    .await
    .map_err(|e| tracing::warn!(%e, "Failed to inject system message into chat"));

    // If approved, emit DealReached so the order is created automatically
    if let Some(price) = final_price_for_deal {
        let chat_event = crate::services::BusinessEvent::DealReached {
            listing_id: listing_id.clone(),
            buyer_id,
            seller_id: user_id,
            final_price: price,
        };
        if let Err(e) = state.event_tx.send(chat_event).await {
            tracing::error!(%e, "Failed to emit DealReached after negotiation approval");
        }
    }

    Ok(Json(NegotiationResponseResult {
        status: new_status.to_string(),
        message: format!("议价请求已更新为 {}", new_status),
    }))
}

#[derive(Deserialize)]
pub struct NegotiationResponse {
    pub action: String,
    #[serde(default)]
    pub counter_price: Option<i64>,
}

#[derive(Serialize)]
pub struct NegotiationResponseResult {
    pub status: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hitl_request_item_serialization() {
        let item = HitlRequestItem {
            id: "req-1".to_string(),
            listing_id: "listing-1".to_string(),
            buyer_id: "buyer-1".to_string(),
            seller_id: "seller-1".to_string(),
            proposed_price: 180.50,
            reason: "Too expensive".to_string(),
            status: "pending".to_string(),
            counter_price: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"proposed_price\":180.5"));
        assert!(json.contains("\"status\":\"pending\""));
    }

    #[test]
    fn test_negotiation_response_deserialize_approve() {
        let json = r#"{"action": "approve"}"#;
        let resp: NegotiationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.action, "approve");
        assert_eq!(resp.counter_price, None);
    }

    #[test]
    fn test_negotiation_response_deserialize_counter() {
        let json = r#"{"action": "counter", "counter_price": 170000}"#;
        let resp: NegotiationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.action, "counter");
        assert_eq!(resp.counter_price, Some(170000));
    }

    #[test]
    fn test_negotiation_response_result_serialization() {
        let result = NegotiationResponseResult {
            status: "approved".to_string(),
            message: "议价请求已更新为 approved".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("approved"));
    }
}
