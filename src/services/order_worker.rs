//! Order lifecycle background worker.
//! 
//! Handles automated transitions:
//! 1. Payment Timeout: pending -> cancelled (30 min)
//! 2. Auto-Completion: shipped -> completed (7 days)

use crate::services::notification::NotificationBroadcast;
use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

pub async fn run(db_pool: PgPool, broadcast: NotificationBroadcast) {
    tracing::info!("Order lifecycle worker started (interval: 5 min)");
    let mut ticker = interval(Duration::from_secs(5 * 60));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        
        // 1. Process payment timeouts
        if let Err(e) = process_payment_timeouts(&db_pool, &broadcast).await {
            tracing::error!(%e, "Payment timeout scan failed");
        }

        // 2. Process auto-completions
        if let Err(e) = process_auto_completions(&db_pool, &broadcast).await {
            tracing::error!(%e, "Auto-completion scan failed");
        }
    }
}

/// Cancel orders that haven't been paid within 30 minutes.
async fn process_payment_timeouts(db: &PgPool, broadcast: &NotificationBroadcast) -> anyhow::Result<()> {
    let expired_rows = sqlx::query(
        r#"
        SELECT id, listing_id, buyer_id, seller_id 
        FROM orders 
        WHERE status = 'pending' AND created_at < NOW() - INTERVAL '30 minutes'
        FOR UPDATE SKIP LOCKED
        "#
    )
    .fetch_all(db)
    .await?;

    for row in expired_rows {
        let order_id: String = row.get("id");
        let listing_id: String = row.get("listing_id");
        let buyer_id: String = row.get("buyer_id");

        // 1. Update order status
        sqlx::query(
            "UPDATE orders SET status = 'cancelled', cancellation_reason = '超时未支付', cancelled_at = NOW() WHERE id = $1"
        )
        .bind(&order_id)
        .execute(db)
        .await?;

        // 2. Re-list the item (crucial!)
        sqlx::query(
            "UPDATE inventory SET status = 'active' WHERE id = $1 AND status = 'sold'"
        )
        .bind(&listing_id)
        .execute(db)
        .await?;

        // 3. Notify buyer
        let notif_id = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            r#"INSERT INTO notifications (id, user_id, event_type, title, body, related_listing_id)
               VALUES ($1, $2, 'order_cancelled_timeout', '订单已自动取消', '由于您未在30分钟内完成支付，订单已自动取消，商品重新上架', $3)"#
        )
        .bind(&notif_id)
        .bind(&buyer_id)
        .bind(&listing_id)
        .execute(db)
        .await;

        let payload = serde_json::json!({
            "id": notif_id,
            "event_type": "order_cancelled_timeout",
            "title": "订单已自动取消",
            "body": "由于您未在30分钟内完成支付，订单已自动取消，商品重新上架",
        });
        broadcast(buyer_id, payload.to_string());
        
        tracing::info!(%order_id, "Order cancelled due to payment timeout");
    }

    Ok(())
}

/// Complete orders that have been shipped for more than 7 days.
async fn process_auto_completions(db: &PgPool, broadcast: &NotificationBroadcast) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT id, buyer_id, seller_id 
        FROM orders 
        WHERE status = 'shipped' AND shipped_at < NOW() - INTERVAL '7 days'
        FOR UPDATE SKIP LOCKED
        "#
    )
    .fetch_all(db)
    .await?;

    for row in rows {
        let order_id: String = row.get("id");
        let buyer_id: String = row.get("buyer_id");
        let seller_id: String = row.get("seller_id");

        sqlx::query(
            "UPDATE orders SET status = 'completed', completed_at = NOW() WHERE id = $1"
        )
        .bind(&order_id)
        .execute(db)
        .await?;

        // Notify both parties
        let msg = "系统已为您自动确认收货，订单已完成";
        
        for uid in &[&buyer_id, &seller_id] {
            let notif_id = Uuid::new_v4().to_string();
            let _ = sqlx::query(
                r#"INSERT INTO notifications (id, user_id, event_type, title, body)
                   VALUES ($1, $2, 'order_auto_completed', '订单已自动完成', $3)"#
            )
            .bind(&notif_id)
            .bind(*uid)
            .bind(msg)
            .execute(db)
            .await;

            broadcast((*uid).clone(), serde_json::json!({
                "id": notif_id,
                "event_type": "order_auto_completed",
                "title": "订单已自动完成",
                "body": msg,
            }).to_string());
        }

        tracing::info!(%order_id, "Order auto-completed after 7 days");
    }

    Ok(())
}
