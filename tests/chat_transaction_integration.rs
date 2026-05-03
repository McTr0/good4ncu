//! Integration tests for chat transaction boundaries.

use good4ncu::repositories::PostgresChatRepository;
use good4ncu::test_infra::with_test_pool;
use sqlx::Row;
use uuid::Uuid;

async fn insert_user(pool: &sqlx::PgPool, id: &str, username: &str) {
    sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, 'hash')")
        .bind(id)
        .bind(username)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_connected_conversation(
    pool: &sqlx::PgPool,
    connection_id: Uuid,
    requester_id: &str,
    receiver_id: &str,
    unread_count: i32,
) {
    sqlx::query(
        "INSERT INTO chat_connections (id, requester_id, receiver_id, status, unread_count, established_at) \
         VALUES ($1, $2, $3, 'connected', $4, NOW())",
    )
    .bind(connection_id)
    .bind(requester_id)
    .bind(receiver_id)
    .bind(unread_count)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_connection(
    pool: &sqlx::PgPool,
    connection_id: Uuid,
    requester_id: &str,
    receiver_id: &str,
    status: &str,
    established_at: Option<chrono::DateTime<chrono::Utc>>,
) {
    sqlx::query(
        "INSERT INTO chat_connections (id, requester_id, receiver_id, status, established_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(connection_id)
    .bind(requester_id)
    .bind(receiver_id)
    .bind(status)
    .bind(established_at)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn create_direct_message_starts_unread_and_increments_unread_count() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-sender", "sender").await;
        insert_user(&pool, "user-receiver", "receiver").await;

        let connection_id = Uuid::new_v4();
        insert_connected_conversation(&pool, connection_id, "user-sender", "user-receiver", 0)
            .await;

        let repo = PostgresChatRepository::new(pool.clone());
        let (message_id, _timestamp, read_at) = repo
            .create_direct_message(
                &connection_id.to_string(),
                Some(connection_id),
                "user-sender",
                Some("user-receiver"),
                "hello",
                None,
                None,
                Some("https://cdn.example.com/a.jpg"),
                None,
                None,
            )
            .await
            .unwrap();

        assert!(message_id > 0);
        assert!(read_at.is_none());

        let message = sqlx::query(
            "SELECT content, sender, receiver, image_url, read_at, read_by, status FROM chat_messages WHERE id = $1",
        )
        .bind(message_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(message.get::<String, _>("content"), "hello");
        assert_eq!(message.get::<String, _>("sender"), "user-sender");
        assert_eq!(
            message.get::<Option<String>, _>("receiver").as_deref(),
            Some("user-receiver")
        );
        assert_eq!(
            message.get::<Option<String>, _>("image_url").as_deref(),
            Some("https://cdn.example.com/a.jpg")
        );
        assert!(message
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("read_at")
            .is_none());
        assert!(message.get::<Option<String>, _>("read_by").is_none());
        assert_eq!(message.get::<String, _>("status"), "sent");

        let unread_count: i32 =
            sqlx::query_scalar("SELECT unread_count FROM chat_connections WHERE id = $1")
                .bind(connection_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(unread_count, 1);
    })
    .await;
}

#[tokio::test]
async fn upsert_connection_request_resets_existing_pair_to_pending() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;

        let connection_id = Uuid::new_v4();
        insert_connection(
            &pool,
            connection_id,
            "user-b",
            "user-a",
            "rejected",
            Some(chrono::Utc::now()),
        )
        .await;

        let repo = PostgresChatRepository::new(pool.clone());
        let result = repo
            .upsert_connection_request("user-a", "user-b")
            .await
            .unwrap();

        assert_eq!(result.connection_id, connection_id.to_string());

        let row = sqlx::query(
            "SELECT requester_id, receiver_id, status, established_at \
             FROM chat_connections WHERE id = $1",
        )
        .bind(connection_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.get::<String, _>("requester_id"), "user-a");
        assert_eq!(row.get::<String, _>("receiver_id"), "user-b");
        assert_eq!(row.get::<String, _>("status"), "pending");
        assert!(row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("established_at")
            .is_none());
    })
    .await;
}

#[tokio::test]
async fn accept_pending_connection_updates_status_and_timestamp() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;

        let connection_id = Uuid::new_v4();
        insert_connection(&pool, connection_id, "user-a", "user-b", "pending", None).await;

        let repo = PostgresChatRepository::new(pool.clone());
        let result = repo
            .accept_pending_connection(&connection_id.to_string(), "user-b")
            .await
            .unwrap();

        assert_eq!(result.requester_id, "user-a");
        assert_eq!(result.receiver_id, "user-b");
        assert!(result.established_at.is_some());

        let row = sqlx::query("SELECT status, established_at FROM chat_connections WHERE id = $1")
            .bind(connection_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<String, _>("status"), "connected");
        assert!(row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("established_at")
            .is_some());
    })
    .await;
}

#[tokio::test]
async fn accept_pending_connection_forbidden_leaves_row_unchanged() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;
        insert_user(&pool, "user-c", "carol").await;

        let connection_id = Uuid::new_v4();
        insert_connection(&pool, connection_id, "user-a", "user-b", "pending", None).await;

        let repo = PostgresChatRepository::new(pool.clone());
        let error = repo
            .accept_pending_connection(&connection_id.to_string(), "user-c")
            .await
            .unwrap_err();
        assert!(matches!(error, good4ncu::api::error::ApiError::Forbidden));

        let row = sqlx::query("SELECT status, established_at FROM chat_connections WHERE id = $1")
            .bind(connection_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<String, _>("status"), "pending");
        assert!(row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("established_at")
            .is_none());
    })
    .await;
}

#[tokio::test]
async fn reject_pending_connection_updates_status() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;

        let connection_id = Uuid::new_v4();
        insert_connection(&pool, connection_id, "user-a", "user-b", "pending", None).await;

        let repo = PostgresChatRepository::new(pool.clone());
        let result = repo
            .reject_pending_connection(&connection_id.to_string(), "user-b")
            .await
            .unwrap();

        assert_eq!(result.requester_id, "user-a");
        assert_eq!(result.receiver_id, "user-b");
        assert!(result.established_at.is_none());

        let status: String =
            sqlx::query_scalar("SELECT status FROM chat_connections WHERE id = $1")
                .bind(connection_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "rejected");
    })
    .await;
}

#[tokio::test]
async fn reject_pending_connection_bad_status_leaves_row_unchanged() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;

        let connection_id = Uuid::new_v4();
        insert_connection(
            &pool,
            connection_id,
            "user-a",
            "user-b",
            "connected",
            Some(chrono::Utc::now()),
        )
        .await;

        let repo = PostgresChatRepository::new(pool.clone());
        let error = repo
            .reject_pending_connection(&connection_id.to_string(), "user-b")
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            good4ncu::api::error::ApiError::BadRequest(_)
        ));

        let row = sqlx::query("SELECT status, established_at FROM chat_connections WHERE id = $1")
            .bind(connection_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<String, _>("status"), "connected");
        assert!(row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("established_at")
            .is_some());
    })
    .await;
}

#[tokio::test]
async fn update_direct_message_content_persists_new_content_and_timestamp() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;

        let connection_id = Uuid::new_v4();
        insert_connected_conversation(&pool, connection_id, "user-a", "user-b", 0).await;

        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, status) \
             VALUES ($1::text, 'direct', 'user-a', 'user-b', false, 'before', 'sent') RETURNING id",
        )
        .bind(connection_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        let edited_at = chrono::Utc::now();
        let repo = PostgresChatRepository::new(pool.clone());
        repo.update_direct_message_content(message_id, "after", edited_at)
            .await
            .unwrap();

        let row = sqlx::query("SELECT content, edited_at FROM chat_messages WHERE id = $1")
            .bind(message_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<String, _>("content"), "after");
        assert_eq!(
            row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("edited_at"),
            Some(edited_at)
        );
    })
    .await;
}

#[tokio::test]
async fn list_user_chat_messages_returns_message_metadata_and_total() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;

        let connection_id = Uuid::new_v4();
        insert_connected_conversation(&pool, connection_id, "user-a", "user-b", 0).await;

        sqlx::query(
            "INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, image_url, read_at, read_by, edited_at, status) \
             VALUES ($1::text, 'direct', 'user-a', 'user-b', false, 'first', 'https://cdn.example.com/one.jpg', NOW(), 'user-b', NOW(), 'read'), \
                    ($1::text, 'direct', 'user-b', 'user-a', true, 'second', NULL, NULL, NULL, NULL, 'sent')",
        )
        .bind(connection_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let repo = PostgresChatRepository::new(pool.clone());
        let (messages, total) = repo
            .list_user_chat_messages(&connection_id.to_string(), 10, 0)
            .await
            .unwrap();

        assert_eq!(total, 2);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "second");
        assert_eq!(messages[0].status, "sent");
        assert!(messages[0].read_at.is_none());
        assert_eq!(messages[1].content, "first");
        assert_eq!(messages[1].status, "read");
        assert_eq!(messages[1].read_by.as_deref(), Some("user-b"));
        assert_eq!(
            messages[1].image_url.as_deref(),
            Some("https://cdn.example.com/one.jpg")
        );
        assert!(messages[1].edited_at.is_some());
    })
    .await;
}

#[tokio::test]
async fn mark_connection_read_with_count_marks_messages_and_resets_unread_count() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "sender").await;
        insert_user(&pool, "user-b", "receiver").await;

        let connection_id = Uuid::new_v4();
        insert_connected_conversation(&pool, connection_id, "user-a", "user-b", 2).await;

        sqlx::query(
            "INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, status) \
             VALUES ($1::text, 'direct', 'user-a', 'user-b', false, 'm1', 'sent'), \
                    ($1::text, 'direct', 'user-a', 'user-b', false, 'm2', 'delivered')",
        )
        .bind(connection_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let repo = PostgresChatRepository::new(pool.clone());
        let now = chrono::Utc::now();
        let marked = repo
            .mark_connection_read_with_count(
                &connection_id.to_string(),
                Some(connection_id),
                "user-b",
                now,
            )
            .await
            .unwrap();

        assert_eq!(marked, 2);

        let unread_count: i32 =
            sqlx::query_scalar("SELECT unread_count FROM chat_connections WHERE id = $1")
                .bind(connection_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(unread_count, 0);

        let rows = sqlx::query(
            "SELECT status, read_by, read_at FROM chat_messages WHERE conversation_id = $1::text ORDER BY id",
        )
        .bind(connection_id.to_string())
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row.get::<String, _>("status"), "read");
            assert_eq!(row.get::<Option<String>, _>("read_by").as_deref(), Some("user-b"));
            assert!(row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("read_at").is_some());
        }
    })
    .await;
}

#[tokio::test]
async fn mark_direct_message_read_preserves_remaining_unread_count() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "sender").await;
        insert_user(&pool, "user-b", "receiver").await;

        let connection_id = Uuid::new_v4();
        insert_connected_conversation(&pool, connection_id, "user-a", "user-b", 2).await;

        sqlx::query(
            "INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, status) \
             VALUES ($1::text, 'direct', 'user-a', 'user-b', false, 'm1', 'sent'), \
                    ($1::text, 'direct', 'user-a', 'user-b', false, 'm2', 'sent')",
        )
        .bind(connection_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let first_message_id: i64 = sqlx::query_scalar(
            "SELECT id FROM chat_messages WHERE conversation_id = $1::text ORDER BY id ASC LIMIT 1",
        )
        .bind(connection_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        let repo = PostgresChatRepository::new(pool.clone());
        repo.mark_direct_message_read(
            first_message_id,
            &connection_id.to_string(),
            Some(connection_id),
            "user-b",
            chrono::Utc::now(),
        )
        .await
        .unwrap();

        let unread_count: i32 =
            sqlx::query_scalar("SELECT unread_count FROM chat_connections WHERE id = $1")
                .bind(connection_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(unread_count, 1);

        let statuses = sqlx::query(
            "SELECT id, read_by, read_at FROM chat_messages WHERE conversation_id = $1::text ORDER BY id ASC",
        )
        .bind(connection_id.to_string())
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(
            statuses[0]
                .get::<Option<String>, _>("read_by")
                .as_deref(),
            Some("user-b")
        );
        assert!(statuses[0]
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("read_at")
            .is_some());
        assert!(statuses[1]
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("read_at")
            .is_none());
    })
    .await;
}
