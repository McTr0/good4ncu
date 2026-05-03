use super::*;
use crate::test_infra::with_test_pool;
use uuid::Uuid;

#[test]
fn test_connection_entry_serialization() {
    let entry = ConnectionEntry {
        id: "conn-1".to_string(),
        requester_id: "user-1".to_string(),
        other_user_id: "user-2".to_string(),
        other_username: Some("alice".to_string()),
        status: "connected".to_string(),
        established_at: Some("2024-01-01T00:00:00Z".to_string()),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        unread_count: 3,
        is_receiver: false,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("conn-1"));
    assert!(json.contains("connected"));
    assert!(json.contains("alice"));
    assert!(json.contains("\"unread_count\":3"));
}

#[test]
fn test_connection_entry_json_structure() {
    let entry = ConnectionEntry {
        id: "conn-123".to_string(),
        requester_id: "user-a".to_string(),
        other_user_id: "user-b".to_string(),
        other_username: Some("bob".to_string()),
        status: "pending".to_string(),
        established_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        unread_count: 0,
        is_receiver: true,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains(r#""id":"conn-123""#));
    assert!(json.contains(r#""requester_id":"user-a""#));
    assert!(json.contains(r#""other_user_id":"user-b""#));
    assert!(json.contains(r#""other_username":"bob""#));
    assert!(json.contains(r#""status":"pending""#));
    assert!(json.contains(r#""established_at":null"#));
    assert!(json.contains(r#""created_at":"2024-01-01T00:00:00Z""#));
    assert!(json.contains(r#""unread_count":0"#));
    assert!(json.contains(r#""is_receiver":true"#));
}

#[test]
fn test_connection_entry_without_username() {
    let entry = ConnectionEntry {
        id: "conn-1".to_string(),
        requester_id: "user-1".to_string(),
        other_user_id: "user-2".to_string(),
        other_username: None,
        status: "pending".to_string(),
        established_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        unread_count: 5,
        is_receiver: true,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("\"other_username\":null"));
}

#[test]
fn test_message_entry_serialization() {
    let entry = MessageEntry {
        id: 1,
        sender: "user-1".to_string(),
        sender_username: Some("alice".to_string()),
        content: "Hello!".to_string(),
        is_agent: false,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        read_by: None,
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("Hello!"));
    assert!(json.contains("user-1"));
    assert!(json.contains("\"is_agent\":false"));
    assert!(json.contains("\"status\":\"sent\""));
}

#[test]
fn test_message_entry_json_structure() {
    let entry = MessageEntry {
        id: 42,
        sender: "user-1".to_string(),
        sender_username: Some("alice".to_string()),
        content: "Test message".to_string(),
        is_agent: false,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        read_by: None,
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains(r#""id":42"#));
    assert!(json.contains(r#""sender":"user-1""#));
    assert!(json.contains(r#""sender_username":"alice""#));
    assert!(json.contains(r#""content":"Test message""#));
    assert!(json.contains(r#""is_agent":false"#));
    assert!(json.contains(r#""status":"sent""#));
}

#[test]
fn test_message_entry_with_read_status_json() {
    let entry = MessageEntry {
        id: 100,
        sender: "user-2".to_string(),
        sender_username: Some("bob".to_string()),
        content: "Read message".to_string(),
        is_agent: false,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: Some("2024-01-01T00:01:00Z".to_string()),
        read_by: Some("user-1".to_string()),
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "read".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains(r#""id":100"#));
    assert!(json.contains(r#""read_at":"2024-01-01T00:01:00Z""#));
    assert!(json.contains(r#""read_by":"user-1""#));
    assert!(json.contains(r#""status":"read""#));
}

#[test]
fn test_message_entry_agent_message() {
    let entry = MessageEntry {
        id: 1,
        sender: "assistant".to_string(),
        sender_username: None,
        content: "AI response".to_string(),
        is_agent: true,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        read_by: None,
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "delivered".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("\"is_agent\":true"));
    assert!(json.contains("assistant"));
}

#[test]
fn test_message_entry_with_image_data() {
    let entry = MessageEntry {
        id: 1,
        sender: "user-1".to_string(),
        sender_username: Some("alice".to_string()),
        content: "Check this out!".to_string(),
        is_agent: false,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        read_by: None,
        image_data: Some("data:image/png;base64,abc123".to_string()),
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("data:image/png;base64,abc123"));
}

#[test]
fn test_message_entry_with_audio_data() {
    let entry = MessageEntry {
        id: 1,
        sender: "user-1".to_string(),
        sender_username: Some("alice".to_string()),
        content: "Voice message".to_string(),
        is_agent: false,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        read_by: None,
        image_data: None,
        audio_data: Some("data:audio/webm;base64,xyz789".to_string()),
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("data:audio/webm;base64,xyz789"));
}

#[test]
fn test_connect_request_response() {
    let resp = ConnectRequestResponse {
        connection_id: "conn-123".to_string(),
        status: "pending".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("conn-123"));
    assert!(json.contains("pending"));
}

#[test]
fn test_connect_request_response_json_structure() {
    let resp = ConnectRequestResponse {
        connection_id: "conn-abc".to_string(),
        status: "pending".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains(r#""connection_id":"conn-abc""#));
    assert!(json.contains(r#""status":"pending""#));
}

#[test]
fn test_connect_accept_response() {
    let resp = ConnectAcceptResponse {
        status: "connected".to_string(),
        established_at: "2024-01-01T12:00:00Z".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("connected"));
    assert!(json.contains("2024-01-01T12:00:00Z"));
}

#[test]
fn test_connect_reject_response() {
    let resp = ConnectRejectResponse {
        status: "rejected".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("rejected"));
}

#[test]
fn test_connect_request_body_deserialization() {
    let json = r#"{"receiver_id": "user-123", "listing_id": "listing-456"}"#;
    let body: ConnectRequestBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.receiver_id, "user-123");
    assert_eq!(body.listing_id, Some("listing-456".to_string()));
}

#[test]
fn test_connect_request_body_without_listing() {
    let json = r#"{"receiver_id": "user-123"}"#;
    let body: ConnectRequestBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.receiver_id, "user-123");
    assert!(body.listing_id.is_none());
}

#[test]
fn test_connect_accept_body_deserialization() {
    let json = r#"{"connection_id": "conn-xyz"}"#;
    let body: ConnectAcceptBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.connection_id, "conn-xyz");
}

#[test]
fn test_connect_reject_body_deserialization() {
    let json = r#"{"connection_id": "conn-xyz"}"#;
    let body: ConnectRejectBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.connection_id, "conn-xyz");
}

#[test]
fn test_send_message_body_deserialization() {
    let json = r#"{"content": "Hello!", "image_base64": null, "audio_base64": null, "image_url": null, "audio_url": null}"#;
    let body: SendMessageBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.content, "Hello!");
    assert!(body.image_base64.is_none());
    assert!(body.audio_base64.is_none());
    assert!(body.image_url.is_none());
    assert!(body.audio_url.is_none());
}

#[test]
fn test_send_message_body_with_media() {
    let json = r#"{
        "content": "Image message",
        "image_base64": "base64data",
        "audio_base64": null,
        "image_url": "https://cdn.example.com/i.jpg",
        "audio_url": null
    }"#;
    let body: SendMessageBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.content, "Image message");
    assert_eq!(body.image_base64, Some("base64data".to_string()));
    assert_eq!(
        body.image_url,
        Some("https://cdn.example.com/i.jpg".to_string())
    );
}

#[test]
fn test_send_message_response() {
    let resp = SendMessageResponse {
        message_id: 42,
        sender: "user-1".to_string(),
        content: "hello".to_string(),
        conversation_id: "conv-1".to_string(),
        sent_at: "2024-01-01T00:00:00Z".to_string(),
        read_at: Some("2024-01-01T00:00:01Z".to_string()),
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("42"));
    assert!(json.contains("2024-01-01T00:00:00Z"));
    assert!(json.contains("\"status\":\"sent\""));
}

#[test]
fn test_send_message_response_id_field_renamed() {
    let resp = SendMessageResponse {
        message_id: 99,
        sender: "user-1".to_string(),
        content: "test".to_string(),
        conversation_id: "conv-1".to_string(),
        sent_at: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sending".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"id\":99"));
}

#[test]
fn test_send_message_response_timestamp_field_renamed() {
    let resp = SendMessageResponse {
        message_id: 1,
        sender: "user-1".to_string(),
        content: "test".to_string(),
        conversation_id: "conv-1".to_string(),
        sent_at: "2024-01-01T12:34:56Z".to_string(),
        read_at: None,
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"timestamp\":\"2024-01-01T12:34:56Z\""));
}

#[test]
fn test_send_message_response_includes_url_fields() {
    let resp = SendMessageResponse {
        message_id: 7,
        sender: "user-1".to_string(),
        content: "with url".to_string(),
        conversation_id: "conv-1".to_string(),
        sent_at: "2024-01-01T12:34:56Z".to_string(),
        read_at: None,
        image_data: None,
        audio_data: None,
        image_url: Some("https://cdn.example.com/a.jpg".to_string()),
        audio_url: Some("https://cdn.example.com/b.m4a".to_string()),
        status: "sent".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"image_url\":\"https://cdn.example.com/a.jpg\""));
    assert!(json.contains("\"audio_url\":\"https://cdn.example.com/b.m4a\""));
}

#[test]
fn test_mark_read_response() {
    let resp = MarkReadResponse {
        message_id: 10,
        read_at: "2024-01-01T12:00:00Z".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("10"));
    assert!(json.contains("2024-01-01T12:00:00Z"));
}

#[test]
fn test_edit_message_body_deserialization() {
    let json = r#"{"content": "Updated message"}"#;
    let body: EditMessageBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.content, "Updated message");
}

#[test]
fn test_edit_message_response_serialization() {
    let resp = EditMessageResponse {
        message_id: 42,
        content: "Updated".to_string(),
        edited_at: "2024-01-01T12:00:00Z".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("42"));
    assert!(json.contains("Updated"));
    assert!(json.contains("2024-01-01T12:00:00Z"));
}

#[test]
fn test_typing_body_deserialization() {
    let json = r#"{"conversation_id": "conv-123"}"#;
    let body: TypingBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.conversation_id, "conv-123");
}

#[test]
fn test_message_list_query_defaults() {
    let query = MessageListQuery {
        limit: None,
        offset: None,
    };
    assert!(query.limit.is_none());
    assert!(query.offset.is_none());
}

#[test]
fn test_message_list_query_with_pagination() {
    let query = MessageListQuery {
        limit: Some(100),
        offset: Some(50),
    };
    assert_eq!(query.limit, Some(100));
    assert_eq!(query.offset, Some(50));
}

#[test]
fn test_connection_list_response() {
    let resp = ConnectionListResponse { items: vec![] };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"items\":[]"));
}

#[test]
fn test_connection_list_response_with_items() {
    let entry = ConnectionEntry {
        id: "conn-1".to_string(),
        requester_id: "user-1".to_string(),
        other_user_id: "user-2".to_string(),
        other_username: Some("alice".to_string()),
        status: "connected".to_string(),
        established_at: Some("2024-01-01T00:00:00Z".to_string()),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        unread_count: 3,
        is_receiver: false,
    };
    let resp = ConnectionListResponse { items: vec![entry] };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("conn-1"));
    assert!(json.contains("\"unread_count\":3"));
}

#[test]
fn test_message_list_response() {
    let resp = MessageListResponse {
        conversation_id: "conn-1".to_string(),
        messages: vec![],
        total: 0,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("conn-1"));
    assert!(json.contains("\"messages\":[]"));
}

#[test]
fn test_message_list_response_with_total() {
    let resp = MessageListResponse {
        conversation_id: "conn-1".to_string(),
        messages: vec![],
        total: 100,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"total\":100"));
}

#[test]
fn test_empty_content_in_send_message_body() {
    let json = r#"{"content": "", "image_base64": null, "audio_base64": null}"#;
    let body: SendMessageBody = serde_json::from_str(json).unwrap();
    assert_eq!(body.content, "");
}

#[test]
fn test_unicode_content_in_message() {
    let entry = MessageEntry {
        id: 1,
        sender: "user-1".to_string(),
        sender_username: Some("中文用户".to_string()),
        content: "你好世界！".to_string(),
        is_agent: false,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        read_by: None,
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("你好世界！"));
    assert!(json.contains("中文用户"));
}

#[test]
fn test_emoji_in_message_content() {
    let entry = MessageEntry {
        id: 1,
        sender: "user-1".to_string(),
        sender_username: None,
        content: "Hello 👋🎉".to_string(),
        is_agent: false,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        read_by: None,
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("Hello 👋🎉"));
    assert!(json.contains(r#""content":"Hello 👋🎉""#));
}

#[test]
fn test_long_content_in_message() {
    let long_content = "a".repeat(2000);
    let entry = MessageEntry {
        id: 1,
        sender: "user-1".to_string(),
        sender_username: None,
        content: long_content.clone(),
        is_agent: false,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        read_at: None,
        read_by: None,
        image_data: None,
        audio_data: None,
        image_url: None,
        audio_url: None,
        status: "sent".to_string(),
        edited_at: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.len() > 2000);
    assert!(json.contains(&"a".repeat(100)));
}

#[test]
fn test_special_characters_in_connection_id() {
    let entry = ConnectionEntry {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        requester_id: "user-1".to_string(),
        other_user_id: "user-2".to_string(),
        other_username: None,
        status: "connected".to_string(),
        established_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        unread_count: 0,
        is_receiver: false,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
}

async fn insert_user(pool: &sqlx::PgPool, id: &str, username: &str) {
    sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, 'hash')")
        .bind(id)
        .bind(username)
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
) {
    sqlx::query(
        "INSERT INTO chat_connections (id, requester_id, receiver_id, status) VALUES ($1, $2, $3, $4)",
    )
    .bind(connection_id)
    .bind(requester_id)
    .bind(receiver_id)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_message(
    pool: &sqlx::PgPool,
    connection_id: Uuid,
    sender: &str,
    receiver: &str,
    content: &str,
) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO chat_messages (conversation_id, listing_id, sender, receiver, is_agent, content, status) \
         VALUES ($1::text, 'direct', $2, $3, false, $4, 'sent') RETURNING id",
    )
    .bind(connection_id.to_string())
    .bind(sender)
    .bind(receiver)
    .bind(content)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn test_load_conversation_access_returns_direct_context_for_participant() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;

        let connection_id = Uuid::new_v4();
        insert_connection(&pool, connection_id, "user-a", "user-b", "connected").await;

        let access =
            super::context::load_conversation_access(&pool, &connection_id.to_string(), "user-a")
                .await
                .unwrap();

        match access {
            super::context::ConversationAccess::Direct(connection) => {
                assert_eq!(connection.connection_uuid, connection_id);
                assert_eq!(connection.requester_id, "user-a");
                assert_eq!(connection.receiver_id, "user-b");
                assert_eq!(connection.status, "connected");
                assert_eq!(connection.other_user_id("user-a").unwrap(), "user-b");
            }
            other => panic!("expected direct access, got {other:?}"),
        }
    })
    .await;
}

#[tokio::test]
async fn test_load_conversation_access_rejects_non_participant() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;
        insert_user(&pool, "user-c", "carol").await;

        let connection_id = Uuid::new_v4();
        insert_connection(&pool, connection_id, "user-a", "user-b", "pending").await;

        let error =
            super::context::load_conversation_access(&pool, &connection_id.to_string(), "user-c")
                .await
                .unwrap_err();
        assert!(matches!(error, crate::api::error::ApiError::Forbidden));

        let status: String =
            sqlx::query_scalar("SELECT status FROM chat_connections WHERE id = $1")
                .bind(connection_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "pending");
    })
    .await;
}

#[test]
fn test_load_conversation_access_recognizes_special_conversations() {
    assert!(super::context::is_special_conversation("__agent__"));
    assert!(super::context::is_special_conversation("global"));
    assert!(!super::context::is_special_conversation("not-special"));
}

#[test]
fn test_direct_conversation_access_requires_connected_status() {
    let access = super::context::DirectConversationAccess {
        connection_uuid: Uuid::new_v4(),
        requester_id: "user-a".to_string(),
        receiver_id: "user-b".to_string(),
        status: "pending".to_string(),
    };

    let error = access.ensure_connected().unwrap_err();
    assert!(matches!(error, crate::api::error::ApiError::BadRequest(_)));
}

#[tokio::test]
async fn test_load_direct_message_access_returns_sender_and_status() {
    with_test_pool(|pool| async move {
        insert_user(&pool, "user-a", "alice").await;
        insert_user(&pool, "user-b", "bob").await;

        let connection_id = Uuid::new_v4();
        insert_connection(&pool, connection_id, "user-a", "user-b", "connected").await;
        let message_id = insert_message(&pool, connection_id, "user-a", "user-b", "hello").await;

        let access = super::context::load_direct_message_access(&pool, message_id)
            .await
            .unwrap();

        assert_eq!(access.sender, "user-a");
        assert_eq!(access.receiver.as_deref(), Some("user-b"));
        assert_eq!(access.conversation_id, connection_id.to_string());
        assert_eq!(access.connection_status, "connected");
        assert_eq!(access.read_at, None);
    })
    .await;
}
