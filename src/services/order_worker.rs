//! Order lifecycle background worker.
//!
//! Handles automated transitions:
//! 1. Payment Timeout: pending -> cancelled (30 min)
//! 2. Auto-Completion: shipped -> completed (7 days)

use crate::repositories::{
    OrderTimestampField, PostgresListingRepository, PostgresOrderRepository,
};
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
async fn process_payment_timeouts(
    db: &PgPool,
    broadcast: &NotificationBroadcast,
) -> anyhow::Result<()> {
    let order_repo = PostgresOrderRepository::new(db.clone());
    let listing_repo = PostgresListingRepository::new(db.clone());
    let expired_rows = sqlx::query(
        r#"
        SELECT id, listing_id, buyer_id
        FROM orders 
        WHERE status = 'pending' AND created_at < NOW() - INTERVAL '30 minutes'
        "#,
    )
    .fetch_all(db)
    .await?;

    for row in expired_rows {
        let order_id: String = row.get("id");
        let listing_id: String = row.get("listing_id");
        let buyer_id: String = row.get("buyer_id");

        let process_result: anyhow::Result<Option<bool>> = async {
            let mut tx = db.begin().await?;

            // Keep lock order consistent with other flows by mutating the order first.
            let updated_listing_id = order_repo
                .update_status_in_tx(
                    &mut tx,
                    &order_id,
                    "pending",
                    "cancelled",
                    OrderTimestampField::Cancelled,
                    Some("超时未支付"),
                )
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            let Some(updated_listing_id) = updated_listing_id else {
                return Ok(None);
            };
            if updated_listing_id != listing_id {
                return Err(anyhow::anyhow!(
                    "Order/listing mismatch while processing timeout order"
                ));
            }

            let relisted = listing_repo
                .relist_if_no_open_orders_in_tx(&mut tx, &listing_id)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;

            if !relisted {
                tracing::info!(
                    %order_id,
                    %listing_id,
                    "Cancelled timeout order without relisting due to listing state or another active order"
                );
            }

            tx.commit().await?;
            Ok(Some(relisted))
        }
        .await;

        let relisted = match process_result {
            Ok(Some(relisted)) => relisted,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(
                    %order_id,
                    %listing_id,
                    %error,
                    "Failed to process timeout order"
                );
                continue;
            }
        };

        // 3. Notify buyer
        let notif_id = Uuid::new_v4().to_string();
        let title = "订单已自动取消";
        let body = if relisted {
            "由于您未在30分钟内完成支付，订单已自动取消，商品重新上架"
        } else {
            "由于您未在30分钟内完成支付，订单已自动取消"
        };
        let insert_result = sqlx::query(
            r#"INSERT INTO notifications (id, user_id, event_type, title, body, related_listing_id)
               VALUES ($1, $2, 'order_cancelled_timeout', $3, $4, $5)"#,
        )
        .bind(&notif_id)
        .bind(&buyer_id)
        .bind(title)
        .bind(body)
        .bind(&listing_id)
        .execute(db)
        .await;

        if insert_result.is_ok() {
            let payload = serde_json::json!({
                "id": notif_id,
                "event_type": "order_cancelled_timeout",
                "title": title,
                "body": body,
            });
            broadcast(buyer_id, payload.to_string());
        } else if let Err(error) = insert_result {
            tracing::warn!(%order_id, %error, "Failed to persist timeout notification");
        }
        tracing::info!(%order_id, "Order cancelled due to payment timeout");
    }

    Ok(())
}

/// Complete orders that have been shipped for more than 7 days.
async fn process_auto_completions(
    db: &PgPool,
    broadcast: &NotificationBroadcast,
) -> anyhow::Result<()> {
    let order_repo = PostgresOrderRepository::new(db.clone());
    let rows = sqlx::query(
        r#"
        SELECT id, buyer_id, seller_id 
        FROM orders 
        WHERE status = 'shipped' AND shipped_at < NOW() - INTERVAL '7 days'
        "#,
    )
    .fetch_all(db)
    .await?;

    for row in rows {
        let order_id: String = row.get("id");
        let buyer_id: String = row.get("buyer_id");
        let seller_id: String = row.get("seller_id");

        let mut tx = db.begin().await?;
        let updated_listing_id = order_repo
            .update_status_in_tx(
                &mut tx,
                &order_id,
                "shipped",
                "completed",
                OrderTimestampField::Completed,
                None,
            )
            .await?;
        if updated_listing_id.is_none() {
            tx.rollback().await?;
            continue;
        }
        tx.commit().await?;

        // Notify both parties
        let msg = "系统已为您自动确认收货，订单已完成";

        for uid in &[&buyer_id, &seller_id] {
            let notif_id = Uuid::new_v4().to_string();
            let insert_result = sqlx::query(
                r#"INSERT INTO notifications (id, user_id, event_type, title, body)
                   VALUES ($1, $2, 'order_auto_completed', '订单已自动完成', $3)"#,
            )
            .bind(&notif_id)
            .bind(*uid)
            .bind(msg)
            .execute(db)
            .await;

            if insert_result.is_ok() {
                broadcast(
                    (*uid).clone(),
                    serde_json::json!({
                        "id": notif_id,
                        "event_type": "order_auto_completed",
                        "title": "订单已自动完成",
                        "body": msg,
                    })
                    .to_string(),
                );
            } else if let Err(error) = insert_result {
                tracing::warn!(%order_id, user_id = %uid, %error, "Failed to persist auto-complete notification");
            }
        }

        tracing::info!(%order_id, "Order auto-completed after 7 days");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_infra::with_test_pool;
    use sqlx::Row;
    use std::sync::{Arc, Mutex};

    async fn with_local_test_pool<F, Fut>(test_body: F)
    where
        F: FnOnce(PgPool) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        with_test_pool(test_body).await;
    }

    async fn insert_user(pool: &PgPool, id: &str, username: &str) {
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role) VALUES ($1, $2, $3, 'user')",
        )
        .bind(id)
        .bind(username)
        .bind("pw")
        .execute(pool)
        .await
        .expect("insert user");
    }

    async fn insert_listing(pool: &PgPool, listing_id: &str, owner_id: &str, status: &str) {
        sqlx::query(
            r#"INSERT INTO inventory
               (id, title, category, brand, condition_score, suggested_price_cny, defects, description, owner_id, status)
               VALUES ($1, 'item', 'cat', 'brand', 8, 1000, 'none', 'desc', $2, $3)"#,
        )
        .bind(listing_id)
        .bind(owner_id)
        .bind(status)
        .execute(pool)
        .await
        .expect("insert listing");
    }

    async fn insert_expired_pending_order(
        pool: &PgPool,
        order_id: &str,
        listing_id: &str,
        buyer_id: &str,
        seller_id: &str,
    ) {
        sqlx::query(
            r#"INSERT INTO orders
               (id, listing_id, buyer_id, seller_id, final_price, status, created_at)
               VALUES ($1, $2, $3, $4, 1000, 'pending', NOW() - INTERVAL '31 minutes')"#,
        )
        .bind(order_id)
        .bind(listing_id)
        .bind(buyer_id)
        .bind(seller_id)
        .execute(pool)
        .await
        .expect("insert order");
    }

    async fn insert_shipped_order(
        pool: &PgPool,
        order_id: &str,
        listing_id: &str,
        buyer_id: &str,
        seller_id: &str,
    ) {
        sqlx::query(
            r#"INSERT INTO orders
               (id, listing_id, buyer_id, seller_id, final_price, status, shipped_at)
               VALUES ($1, $2, $3, $4, 1000, 'shipped', NOW() - INTERVAL '8 days')"#,
        )
        .bind(order_id)
        .bind(listing_id)
        .bind(buyer_id)
        .bind(seller_id)
        .execute(pool)
        .await
        .expect("insert shipped order");
    }

    #[tokio::test]
    async fn test_process_payment_timeouts_cancels_order_and_relists_inventory() {
        with_local_test_pool(|pool| async move {
            let seller_id = Uuid::new_v4().to_string();
            let buyer_id = Uuid::new_v4().to_string();
            let listing_id = Uuid::new_v4().to_string();
            let order_id = Uuid::new_v4().to_string();

            insert_user(&pool, &seller_id, "seller_timeout_ok").await;
            insert_user(&pool, &buyer_id, "buyer_timeout_ok").await;
            insert_listing(&pool, &listing_id, &seller_id, "sold").await;
            insert_expired_pending_order(&pool, &order_id, &listing_id, &buyer_id, &seller_id).await;

            let pushes: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let pushes_for_cb = Arc::clone(&pushes);
            let broadcast: NotificationBroadcast = Arc::new(move |uid, _payload| {
                if let Ok(mut lock) = pushes_for_cb.lock() {
                    lock.push(uid);
                }
            });

            process_payment_timeouts(&pool, &broadcast)
                .await
                .expect("process payment timeouts");

            let order_row = sqlx::query(
                "SELECT status, cancellation_reason FROM orders WHERE id = $1",
            )
            .bind(&order_id)
            .fetch_one(&pool)
            .await
            .expect("select order");
            let order_status: String = order_row.get("status");
            let cancellation_reason: Option<String> = order_row.try_get("cancellation_reason").ok();
            assert_eq!(order_status, "cancelled");
            assert_eq!(cancellation_reason.as_deref(), Some("超时未支付"));

            let inventory_row = sqlx::query("SELECT status FROM inventory WHERE id = $1")
                .bind(&listing_id)
                .fetch_one(&pool)
                .await
                .expect("select inventory");
            let inventory_status: String = inventory_row.get("status");
            assert_eq!(inventory_status, "active");

            let notif_count: i64 = sqlx::query(
                "SELECT COUNT(*) AS c FROM notifications WHERE user_id = $1 AND event_type = 'order_cancelled_timeout'",
            )
            .bind(&buyer_id)
            .fetch_one(&pool)
            .await
            .expect("count notifications")
            .get("c");
            assert_eq!(notif_count, 1);

            let notif_body: String = sqlx::query(
                "SELECT body FROM notifications WHERE user_id = $1 AND event_type = 'order_cancelled_timeout'",
            )
            .bind(&buyer_id)
            .fetch_one(&pool)
            .await
            .expect("select notification body")
            .get("body");
            assert_eq!(
                notif_body,
                "由于您未在30分钟内完成支付，订单已自动取消，商品重新上架"
            );

            let pushed_users = pushes.lock().expect("broadcast mutex lock").clone();
            assert_eq!(pushed_users, vec![buyer_id]);
        })
        .await;
    }

    #[tokio::test]
    async fn test_process_payment_timeouts_keeps_inventory_sold_when_newer_open_order_exists() {
        with_local_test_pool(|pool| async move {
            let seller_id = Uuid::new_v4().to_string();
            let old_buyer_id = Uuid::new_v4().to_string();
            let new_buyer_id = Uuid::new_v4().to_string();
            let listing_id = Uuid::new_v4().to_string();
            let expired_order_id = Uuid::new_v4().to_string();
            let open_order_id = Uuid::new_v4().to_string();

            insert_user(&pool, &seller_id, "seller_timeout_rollback").await;
            insert_user(&pool, &old_buyer_id, "buyer_timeout_old").await;
            insert_user(&pool, &new_buyer_id, "buyer_timeout_new").await;
            insert_listing(&pool, &listing_id, &seller_id, "sold").await;
            insert_expired_pending_order(
                &pool,
                &expired_order_id,
                &listing_id,
                &old_buyer_id,
                &seller_id,
            )
            .await;

            sqlx::query(
                r#"INSERT INTO orders
                   (id, listing_id, buyer_id, seller_id, final_price, status, created_at)
                   VALUES ($1, $2, $3, $4, 1000, 'pending', NOW())"#,
            )
            .bind(&open_order_id)
            .bind(&listing_id)
            .bind(&new_buyer_id)
            .bind(&seller_id)
            .execute(&pool)
            .await
            .expect("insert newer open order");

            let broadcast: NotificationBroadcast = Arc::new(|_, _| {});

            process_payment_timeouts(&pool, &broadcast)
                .await
                .expect("process payment timeouts");

            let expired_order_status: String =
                sqlx::query("SELECT status FROM orders WHERE id = $1")
                .bind(&expired_order_id)
                .fetch_one(&pool)
                .await
                .expect("select expired order status")
                .get("status");
            assert_eq!(expired_order_status, "cancelled");

            let newer_order_status: String = sqlx::query("SELECT status FROM orders WHERE id = $1")
                .bind(&open_order_id)
                .fetch_one(&pool)
                .await
                .expect("select open order status")
                .get("status");
            assert_eq!(newer_order_status, "pending");

            let inventory_status: String = sqlx::query("SELECT status FROM inventory WHERE id = $1")
                .bind(&listing_id)
                .fetch_one(&pool)
                .await
                .expect("select inventory status")
                .get("status");
            assert_eq!(inventory_status, "sold");

            let notif_count: i64 = sqlx::query(
                "SELECT COUNT(*) AS c FROM notifications WHERE user_id = $1 AND event_type = 'order_cancelled_timeout'",
            )
            .bind(&old_buyer_id)
            .fetch_one(&pool)
            .await
            .expect("count notifications")
            .get("c");
            assert_eq!(notif_count, 1);

            let notif_body: String = sqlx::query(
                "SELECT body FROM notifications WHERE user_id = $1 AND event_type = 'order_cancelled_timeout'",
            )
            .bind(&old_buyer_id)
            .fetch_one(&pool)
            .await
            .expect("select notification body")
            .get("body");
            assert_eq!(notif_body, "由于您未在30分钟内完成支付，订单已自动取消");
        })
        .await;
    }

    #[tokio::test]
    async fn test_process_payment_timeouts_is_idempotent_on_repeat_run() {
        with_local_test_pool(|pool| async move {
            let seller_id = Uuid::new_v4().to_string();
            let buyer_id = Uuid::new_v4().to_string();
            let listing_id = Uuid::new_v4().to_string();
            let order_id = Uuid::new_v4().to_string();

            insert_user(&pool, &seller_id, "seller_timeout_idempotent").await;
            insert_user(&pool, &buyer_id, "buyer_timeout_idempotent").await;
            insert_listing(&pool, &listing_id, &seller_id, "sold").await;
            insert_expired_pending_order(&pool, &order_id, &listing_id, &buyer_id, &seller_id).await;

            let pushes: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let pushes_for_cb = Arc::clone(&pushes);
            let broadcast: NotificationBroadcast = Arc::new(move |uid, _payload| {
                if let Ok(mut lock) = pushes_for_cb.lock() {
                    lock.push(uid);
                }
            });

            process_payment_timeouts(&pool, &broadcast)
                .await
                .expect("first timeout run");

            let order_status_after_first: String =
                sqlx::query("SELECT status FROM orders WHERE id = $1")
                    .bind(&order_id)
                    .fetch_one(&pool)
                    .await
                    .expect("select order status after first run")
                    .get("status");
            assert_eq!(order_status_after_first, "cancelled");

            let notif_count_after_first: i64 = sqlx::query(
                "SELECT COUNT(*) AS c FROM notifications WHERE user_id = $1 AND event_type = 'order_cancelled_timeout'",
            )
            .bind(&buyer_id)
            .fetch_one(&pool)
            .await
            .expect("count timeout notifications after first run")
            .get("c");
            assert_eq!(notif_count_after_first, 1);

            let pushes_after_first = pushes.lock().expect("broadcast mutex lock").clone();
            assert_eq!(pushes_after_first, vec![buyer_id.clone()]);

            process_payment_timeouts(&pool, &broadcast)
                .await
                .expect("second timeout run");

            let order_status: String = sqlx::query("SELECT status FROM orders WHERE id = $1")
                .bind(&order_id)
                .fetch_one(&pool)
                .await
                .expect("select order status")
                .get("status");
            assert_eq!(order_status, "cancelled");

            let inventory_status: String = sqlx::query("SELECT status FROM inventory WHERE id = $1")
                .bind(&listing_id)
                .fetch_one(&pool)
                .await
                .expect("select inventory status")
                .get("status");
            assert_eq!(inventory_status, "active");

            let notif_count: i64 = sqlx::query(
                "SELECT COUNT(*) AS c FROM notifications WHERE user_id = $1 AND event_type = 'order_cancelled_timeout'",
            )
            .bind(&buyer_id)
            .fetch_one(&pool)
            .await
            .expect("count timeout notifications")
            .get("c");
            assert_eq!(notif_count, 1);

            let pushes_snapshot = pushes.lock().expect("broadcast mutex lock").clone();
            assert_eq!(pushes_snapshot, vec![buyer_id]);
        })
        .await;
    }

    #[tokio::test]
    async fn test_process_auto_completions_is_idempotent_on_repeat_run() {
        with_local_test_pool(|pool| async move {
            let seller_id = Uuid::new_v4().to_string();
            let buyer_id = Uuid::new_v4().to_string();
            let listing_id = Uuid::new_v4().to_string();
            let order_id = Uuid::new_v4().to_string();

            insert_user(&pool, &seller_id, "seller_complete_idempotent").await;
            insert_user(&pool, &buyer_id, "buyer_complete_idempotent").await;
            insert_listing(&pool, &listing_id, &seller_id, "sold").await;
            insert_shipped_order(&pool, &order_id, &listing_id, &buyer_id, &seller_id).await;

            let pushes: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let pushes_for_cb = Arc::clone(&pushes);
            let broadcast: NotificationBroadcast = Arc::new(move |uid, _payload| {
                if let Ok(mut lock) = pushes_for_cb.lock() {
                    lock.push(uid);
                }
            });

            process_auto_completions(&pool, &broadcast)
                .await
                .expect("first completion run");

            let order_row_after_first =
                sqlx::query("SELECT status, completed_at FROM orders WHERE id = $1")
                    .bind(&order_id)
                    .fetch_one(&pool)
                    .await
                    .expect("select completed order after first run");
            let order_status_after_first: String = order_row_after_first.get("status");
            let completed_at_after_first: chrono::DateTime<chrono::Utc> =
                order_row_after_first
                    .try_get("completed_at")
                    .expect("completed_at should be set after first run");
            assert_eq!(order_status_after_first, "completed");

            let buyer_notif_count_after_first: i64 = sqlx::query(
                "SELECT COUNT(*) AS c FROM notifications WHERE user_id = $1 AND event_type = 'order_auto_completed'",
            )
            .bind(&buyer_id)
            .fetch_one(&pool)
            .await
            .expect("count buyer notifications after first run")
            .get("c");
            assert_eq!(buyer_notif_count_after_first, 1);

            let seller_notif_count_after_first: i64 = sqlx::query(
                "SELECT COUNT(*) AS c FROM notifications WHERE user_id = $1 AND event_type = 'order_auto_completed'",
            )
            .bind(&seller_id)
            .fetch_one(&pool)
            .await
            .expect("count seller notifications after first run")
            .get("c");
            assert_eq!(seller_notif_count_after_first, 1);

            let pushes_after_first = pushes.lock().expect("broadcast mutex lock").clone();
            let buyer_pushes_after_first =
                pushes_after_first.iter().filter(|uid| *uid == &buyer_id).count();
            let seller_pushes_after_first = pushes_after_first
                .iter()
                .filter(|uid| *uid == &seller_id)
                .count();
            assert_eq!(pushes_after_first.len(), 2);
            assert_eq!(buyer_pushes_after_first, 1);
            assert_eq!(seller_pushes_after_first, 1);

            process_auto_completions(&pool, &broadcast)
                .await
                .expect("second completion run");

            let order_row = sqlx::query("SELECT status, completed_at FROM orders WHERE id = $1")
                .bind(&order_id)
                .fetch_one(&pool)
                .await
                .expect("select completed order");
            let order_status: String = order_row.get("status");
            let completed_at: Option<chrono::DateTime<chrono::Utc>> =
                order_row.try_get("completed_at").ok();
            assert_eq!(order_status, "completed");
            assert!(completed_at.is_some());
            assert_eq!(
                completed_at,
                Some(completed_at_after_first),
                "completed_at should remain unchanged on second idempotent run"
            );

            let buyer_notif_count: i64 = sqlx::query(
                "SELECT COUNT(*) AS c FROM notifications WHERE user_id = $1 AND event_type = 'order_auto_completed'",
            )
            .bind(&buyer_id)
            .fetch_one(&pool)
            .await
            .expect("count buyer completion notifications")
            .get("c");
            assert_eq!(buyer_notif_count, 1);

            let seller_notif_count: i64 = sqlx::query(
                "SELECT COUNT(*) AS c FROM notifications WHERE user_id = $1 AND event_type = 'order_auto_completed'",
            )
            .bind(&seller_id)
            .fetch_one(&pool)
            .await
            .expect("count seller completion notifications")
            .get("c");
            assert_eq!(seller_notif_count, 1);

            let pushes_snapshot = pushes.lock().expect("broadcast mutex lock").clone();
            let buyer_pushes = pushes_snapshot.iter().filter(|uid| *uid == &buyer_id).count();
            let seller_pushes = pushes_snapshot.iter().filter(|uid| *uid == &seller_id).count();
            assert_eq!(pushes_snapshot.len(), 2);
            assert_eq!(buyer_pushes, 1);
            assert_eq!(seller_pushes, 1);
        })
        .await;
    }
}
