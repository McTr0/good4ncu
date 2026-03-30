use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ConnectRequestBody {
    pub receiver_id: String,
    pub listing_id: Option<String>,
}

#[derive(Serialize)]
pub struct ConnectRequestResponse {
    pub connection_id: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct ConnectAcceptBody {
    pub connection_id: String,
}

#[derive(Serialize)]
pub struct ConnectAcceptResponse {
    pub status: String,
    pub established_at: String,
}

#[derive(Deserialize)]
pub struct ConnectRejectBody {
    pub connection_id: String,
}

#[derive(Serialize)]
pub struct ConnectRejectResponse {
    pub status: String,
}

#[derive(Serialize)]
pub struct ConnectionEntry {
    pub id: String,
    pub requester_id: String,
    pub other_user_id: String,
    pub other_username: Option<String>,
    pub status: String,
    pub established_at: Option<String>,
    pub created_at: String,
    /// unread message count
    pub unread_count: i32,
    /// Whether the current user is the receiver (can accept/reject this pending request)
    pub is_receiver: bool,
}

#[derive(Serialize)]
pub struct ConnectionListResponse {
    pub items: Vec<ConnectionEntry>,
}

#[derive(Deserialize)]
pub struct SendMessageBody {
    pub content: String,
    pub image_base64: Option<String>,
    pub audio_base64: Option<String>,
    pub image_url: Option<String>,
    pub audio_url: Option<String>,
}

#[derive(Serialize)]
pub struct SendMessageResponse {
    /// Returned as `id` for frontend compatibility with ConversationMessage.fromJson
    #[serde(rename = "id")]
    pub message_id: i64,
    pub sender: String,
    pub content: String,
    pub conversation_id: String,
    #[serde(rename = "timestamp")]
    pub sent_at: String,
    pub read_at: Option<String>,
    pub image_data: Option<String>,
    pub audio_data: Option<String>,
    pub image_url: Option<String>,
    pub audio_url: Option<String>,
    /// message status: sending | sent | delivered | read | failed
    pub status: String,
}

#[derive(Deserialize)]
pub struct MessageListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct MessageEntry {
    pub id: i64,
    pub sender: String,
    pub sender_username: Option<String>,
    pub content: String,
    pub is_agent: bool,
    pub timestamp: String,
    pub read_at: Option<String>,
    pub read_by: Option<String>,
    pub image_data: Option<String>,
    pub audio_data: Option<String>,
    pub image_url: Option<String>,
    pub audio_url: Option<String>,
    /// message edit status: sending | sent | delivered | read | failed
    pub status: String,
    /// edited time
    pub edited_at: Option<String>,
}

#[derive(Serialize)]
pub struct MessageListResponse {
    pub conversation_id: String,
    pub messages: Vec<MessageEntry>,
    pub total: i64,
}

#[derive(Serialize)]
pub struct MarkReadResponse {
    pub message_id: i64,
    pub read_at: String,
}

#[derive(Deserialize)]
pub struct EditMessageBody {
    pub content: String,
}

#[derive(Serialize)]
pub struct EditMessageResponse {
    pub message_id: i64,
    pub content: String,
    pub edited_at: String,
}

#[derive(Deserialize)]
pub struct TypingBody {
    pub conversation_id: String,
}
