//! HITL negotiation timeout background worker.
//!
//! Runs every 10 minutes and expires pending hitl_requests where:
//! - status = 'pending' (seller hasn't responded)
//! - expires_at < NOW() (48 hours have passed)
//!
//! On expiration:
//! 1. Updates status to 'expired' in DB
//! 2. Injects a system message into the conversation
//! 3. Notifies the buyer: "卖家超时未回应，本次议价已自动取消"

use crate::services::notification::NotificationBroadcast;
use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

/// Run the HITL expiration worker.
/// Spawns a background tokio task that scans and expires stale requests.
pub async fn run(db_pool: PgPool, broadcast: NotificationBroadcast) {
    tracing::info!("HITL expiration worker started (interval: 10 min)");
    let mut ticker = interval(Duration::from_secs(10 * 60));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        if let Err(e) = expire_pending(&db_pool, &broadcast).await {
            tracing::error!(%e, "HITL expiration scan failed");
        }
    }
}

/// Scan for and expire all pending hitl_requests past their expires_at.
/// Returns the number of records expired.
async fn expire_pending(
    db_pool: &PgPool,
    broadcast: &NotificationBroadcast,
) -> anyhow::Result<usize> {
    // Fetch all expired pending requests in a single query.
    let rows = sqlx::query(
        r#"
        SELECT id, listing_id, buyer_id, seller_id, proposed_price
        FROM hitl_requests
        WHERE status = 'pending' AND expires_at < NOW()
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    if rows.is_empty() {
        return Ok(0);
    }

    tracing::info!(
        count = rows.len(),
        "Found {} expired HITL requests to process",
        rows.len()
    );

    let mut count = 0;
    for row in &rows {
        let id: String = row.get("id");
        let listing_id: String = row.get("listing_id");
        let buyer_id: String = row.get("buyer_id");
        let seller_id: String = row.get("seller_id");
        let _proposed_price: i64 = row.get::<i32, _>("proposed_price") as i64;

        // Update status to expired.
        let update_result = sqlx::query(
            r#"UPDATE hitl_requests
               SET status = 'expired', resolved_at = NOW()
               WHERE id = $1 AND status = 'pending' AND expires_at < NOW()"#,
        )
        .bind(&id)
        .execute(db_pool)
        .await;

        match update_result {
            Ok(result) if result.rows_affected() == 1 => {
                count += 1;
                tracing::debug!(%id, "Expired HITL request");

                // Inject system message into conversation.
                let conversation_id = format!("negotiate:{}", listing_id);
                let system_content = "系统：卖家超时未回应（48小时内未处理），本次议价已自动取消";
                let _ = sqlx::query(
                    r#"INSERT INTO chat_messages (conversation_id, sender, is_agent, content, listing_id)
                       VALUES ($1, 'system', FALSE, $2, $3)"#,
                )
                .bind(&conversation_id)
                .bind(system_content)
                .bind(&listing_id)
                .execute(db_pool)
                .await
                .map_err(|e| tracing::warn!(%e, "Failed to inject expiration system message"));

                // Notify buyer: seller didn't respond in time.
                let notification_id = Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    r#"INSERT INTO notifications (id, user_id, event_type, title, body, related_listing_id)
                       VALUES ($1, $2, 'negotiation_expired', '议价已超时取消',
                               '卖家超时未回应（48小时内未处理），本次议价已自动取消', $3)"#,
                )
                .bind(&notification_id)
                .bind(&buyer_id)
                .bind(&listing_id)
                .execute(db_pool)
                .await
                .map_err(|e| tracing::warn!(%e, "Failed to notify buyer of expiration"));

                // Push WebSocket notification to buyer immediately.
                let notif_payload = serde_json::json!({
                    "id": notification_id,
                    "event_type": "negotiation_expired",
                    "title": "议价已超时取消",
                    "body": "卖家超时未回应（48小时内未处理），本次议价已自动取消",
                });
                broadcast(buyer_id.clone(), notif_payload.to_string());

                // Notify seller as well — they missed a negotiation request.
                let seller_notif_id = Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    r#"INSERT INTO notifications (id, user_id, event_type, title, body, related_listing_id)
                       VALUES ($1, $2, 'negotiation_expired_seller', '议价超时未处理',
                               '您有一笔议价请求超时未处理，已自动取消', $3)"#,
                )
                .bind(&seller_notif_id)
                .bind(&seller_id)
                .bind(&listing_id)
                .execute(db_pool)
                .await
                .map_err(|e| tracing::warn!(%e, "Failed to notify seller of expiration"));

                let seller_notif_payload = serde_json::json!({
                    "id": seller_notif_id,
                    "event_type": "negotiation_expired_seller",
                    "title": "议价超时未处理",
                    "body": "您有一笔议价请求超时未处理，已自动取消",
                });
                broadcast(seller_id.clone(), seller_notif_payload.to_string());
            }
            Ok(_) => {
                // Already processed by another worker instance — skip
                tracing::debug!(%id, "HITL request already expired by another worker");
            }
            Err(e) => {
                tracing::error!(%e, %id, "Failed to update HITL status to expired");
            }
        }
    }

    if count > 0 {
        tracing::info!(count, "Expired {} HITL negotiation requests", count);
    }

    Ok(count)
}
