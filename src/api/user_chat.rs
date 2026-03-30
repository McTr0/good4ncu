//! User-to-user direct chat with connection handshake.
//!
//! Implements a three-way handshake for establishing chat connections:
//! 1. Requester sends POST /api/chat/connect/request → status=pending
//! 2. Receiver accepts via POST /api/chat/connect/accept → status=connected
//!    (or rejects via POST /api/chat/connect/reject → status=rejected)
//! 3. Once connected, messages can be exchanged via POST /api/chat/conversations/{id}/messages
//!
//! WebSocket events pushed to participants:
//! - `connection_request` — new connection request received
//! - `connection_established` — connection accepted and established
//! - `new_message` — new direct message
//! - `message_read` — a message was marked as read

pub use crate::api::user_chat_models::{
    ConnectAcceptBody, ConnectAcceptResponse, ConnectRejectBody, ConnectRejectResponse,
    ConnectRequestBody, ConnectRequestResponse, ConnectionEntry, ConnectionListResponse,
    EditMessageBody, EditMessageResponse, MarkReadResponse, MessageEntry, MessageListQuery,
    MessageListResponse, SendMessageBody, SendMessageResponse, TypingBody,
};

mod connection;
mod message;
mod events;

pub use connection::{connect_accept, connect_reject, connect_request, list_connections};
pub use message::{
    edit_message, get_connection_messages, mark_connection_read, mark_message_read,
    send_connection_message, typing_indicator,
};

pub(crate) use events::{
    WsConnectionEstablishedEvent, WsConnectionRejectedEvent, WsConnectionRequestEvent,
    WsMessageReadEvent, WsNewMessageEvent, WsTypingEvent,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Request/Response Serialization Tests
    // ========================================================================

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
        // ConnectionEntry only implements Serialize, not Deserialize.
        // Verify the JSON output has correct structure by checking key presence.
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
        // Verify JSON structure contains expected keys
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
        // other_username should be null in JSON, not omitted
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
        // MessageEntry only implements Serialize, not Deserialize.
        // Verify the JSON output has correct structure by serializing and checking keys.
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
        // Verify JSON structure
        assert!(json.contains(r#""id":42"#));
        assert!(json.contains(r#""sender":"user-1""#));
        assert!(json.contains(r#""sender_username":"alice""#));
        assert!(json.contains(r#""content":"Test message""#));
        assert!(json.contains(r#""is_agent":false"#));
        assert!(json.contains(r#""status":"sent""#));
    }

    #[test]
    fn test_message_entry_with_read_status_json() {
        // Verify message with read status serializes correctly
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
        // ConnectRequestResponse only implements Serialize, not Deserialize.
        // Verify the JSON output has correct structure.
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
        // SendMessageResponse uses #[serde(rename = "id")] for message_id
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
        // The message_id field should be serialized as "id" due to #[serde(rename = "id")]
        assert!(json.contains("\"id\":99"));
    }

    #[test]
    fn test_send_message_response_timestamp_field_renamed() {
        // SendMessageResponse uses #[serde(rename = "timestamp")] for sent_at
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
        // The sent_at field should be serialized as "timestamp"
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

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_empty_content_in_send_message_body() {
        // Empty content should be valid for deserialization
        // (validation happens in the handler, not at deserialization)
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
        // MessageEntry only implements Serialize, not Deserialize.
        // Verify serialization preserves emoji content.
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
        // MessageEntry only implements Serialize, not Deserialize.
        // Verify serialization handles long content.
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.len() > 2000);
        // The content should appear in the JSON
        assert!(json.contains(&"a".repeat(100)));
    }

    #[test]
    fn test_special_characters_in_connection_id() {
        // UUIDs should be serializable
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
}
