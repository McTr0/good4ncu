//! End-to-end tests for the private chat flow (user-to-user direct chat).
//!
//! Tests the three-way handshake connection lifecycle, message sending,
//! message editing, read receipts, and typing indicators.
//!
//! Run with: `cargo test --test chat_e2e`
//!
//! Requires a running PostgreSQL database with `DATABASE_URL` env var set.
//! Tests are independent and clean up their data in the drop step.

use axum::http::{HeaderMap, StatusCode};
use good4ncu::test_infra::db_safety;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Test user credentials
struct TestUser {
    user_id: String,
    username: String,
    token: String,
}

/// Registers a new user and returns the auth response.
async fn register_user(
    client: &Client,
    base_url: &str,
    username: &str,
    password: &str,
) -> anyhow::Result<AuthResponse> {
    let response = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&serde_json::json!({
            "username": username,
            "password": password
        }))
        .send()
        .await?;

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Registration failed: {:?}",
        response.text().await?
    );
    Ok(response.json::<AuthResponse>().await?)
}

/// Logs in a user and returns the auth response.
async fn login_user(
    client: &Client,
    base_url: &str,
    username: &str,
    password: &str,
) -> anyhow::Result<AuthResponse> {
    let response = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&serde_json::json!({
            "username": username,
            "password": password
        }))
        .send()
        .await?;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Login failed: {:?}",
        response.text().await?
    );
    Ok(response.json::<AuthResponse>().await?)
}

/// Creates a test user by registering and logging in, returning the user details and token.
async fn create_test_user(
    client: &Client,
    base_url: &str,
    prefix: &str,
) -> anyhow::Result<TestUser> {
    let unique_id = Uuid::new_v4().to_string();
    let unique_suffix = unique_id.split('-').next().unwrap();
    let username = format!("{}_{}", prefix, unique_suffix);
    let password = "testpass123".to_string();

    // Register the user
    let _reg_response = register_user(client, base_url, &username, &password).await?;

    // Log in to get the token
    let login_response = login_user(client, base_url, &username, &password).await?;

    Ok(TestUser {
        user_id: login_response.user_id,
        username,
        token: login_response.token,
    })
}

// ---------------------------------------------------------------------------
// Request/Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub username: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
struct ConnectRequestBody {
    receiver_id: String,
    listing_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConnectRequestResponse {
    pub connection_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
struct ConnectAcceptBody {
    connection_id: String,
}

#[derive(Debug, Deserialize)]
struct ConnectAcceptResponse {
    pub status: String,
    #[allow(dead_code)]
    pub established_at: String,
}

#[derive(Debug, Serialize)]
struct ConnectRejectBody {
    connection_id: String,
}

#[derive(Debug, Deserialize)]
struct ConnectRejectResponse {
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct ConnectionEntry {
    pub id: String,
    #[allow(dead_code)]
    pub requester_id: String,
    #[allow(dead_code)]
    pub other_user_id: String,
    #[allow(dead_code)]
    pub other_username: Option<String>,
    pub status: String,
    #[allow(dead_code)]
    pub established_at: Option<String>,
    #[allow(dead_code)]
    pub created_at: String,
    #[allow(dead_code)]
    pub unread_count: i32,
    pub is_receiver: bool,
}

#[derive(Debug, Deserialize)]
struct ConnectionListResponse {
    pub items: Vec<ConnectionEntry>,
}

#[derive(Debug, Serialize)]
struct SendMessageBody {
    content: String,
    image_base64: Option<String>,
    audio_base64: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    pub message_id: i64,
    pub sender: String,
    pub content: String,
    pub conversation_id: String,
    #[allow(dead_code)]
    pub timestamp: String,
    #[allow(dead_code)]
    pub read_at: Option<String>,
    #[allow(dead_code)]
    pub image_data: Option<String>,
    #[allow(dead_code)]
    pub audio_data: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct MessageEntry {
    pub id: i64,
    pub sender: String,
    pub sender_username: Option<String>,
    pub content: String,
    #[allow(dead_code)]
    pub is_agent: bool,
    pub timestamp: String,
    pub read_at: Option<String>,
    pub read_by: Option<String>,
    #[allow(dead_code)]
    pub image_data: Option<String>,
    #[allow(dead_code)]
    pub audio_data: Option<String>,
    #[allow(dead_code)]
    pub status: String,
    pub edited_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageListResponse {
    #[allow(dead_code)]
    pub conversation_id: String,
    pub messages: Vec<MessageEntry>,
    #[allow(dead_code)]
    pub total: i64,
}

#[derive(Debug, Serialize)]
struct EditMessageBody {
    content: String,
}

#[derive(Debug, Deserialize)]
struct EditMessageResponse {
    #[allow(dead_code)]
    pub message_id: i64,
    pub content: String,
    pub edited_at: String,
}

#[derive(Debug, Deserialize)]
struct MarkReadResponse {
    pub message_id: i64,
    pub read_at: String,
}

#[derive(Debug, Serialize)]
struct TypingBody {
    conversation_id: String,
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn auth_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    headers
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test 1: Connection lifecycle - request, accept, reject
#[tokio::test]
async fn test_connection_lifecycle() -> anyhow::Result<()> {
    let database_url = db_safety::resolve_test_database_url();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create two test users
    let user_a = create_test_user(&client, &base_url, "user_a").await?;
    let user_b = create_test_user(&client, &base_url, "user_b").await?;

    // Clean up any existing connections between these users
    sqlx::query(
        "DELETE FROM chat_connections WHERE (requester_id = $1 AND receiver_id = $2) OR (requester_id = $2 AND receiver_id = $1)"
    )
    .bind(&user_a.user_id)
    .bind(&user_b.user_id)
    .execute(&sqlx::PgPool::connect(&database_url).await?)
    .await?;

    // Step 1: User A sends connection request to User B
    let request_response = client
        .post(format!("{}/api/chat/connect/request", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&ConnectRequestBody {
            receiver_id: user_b.user_id.clone(),
            listing_id: None,
        })
        .send()
        .await?;

    assert_eq!(
        request_response.status(),
        StatusCode::OK,
        "Connect request failed: {:?}",
        request_response.text().await?
    );
    let request_body = request_response.json::<ConnectRequestResponse>().await?;
    assert_eq!(request_body.status, "pending");
    let connection_id = request_body.connection_id;

    // Verify User B can see the pending connection
    let list_response_b = client
        .get(format!("{}/api/chat/connections", base_url))
        .headers(auth_headers(&user_b.token))
        .send()
        .await?;

    assert_eq!(list_response_b.status(), StatusCode::OK);
    let connections_b = list_response_b.json::<ConnectionListResponse>().await?;
    let found_connection = connections_b.items.iter().find(|c| c.id == connection_id);
    assert!(
        found_connection.is_some(),
        "User B should see the pending connection"
    );
    let conn_entry = found_connection.unwrap();
    assert_eq!(conn_entry.status, "pending");
    assert!(conn_entry.is_receiver, "User B should be the receiver");

    // Step 2a: User B accepts the connection
    let accept_response = client
        .post(format!("{}/api/chat/connect/accept", base_url))
        .headers(auth_headers(&user_b.token))
        .json(&ConnectAcceptBody {
            connection_id: connection_id.clone(),
        })
        .send()
        .await?;

    assert_eq!(
        accept_response.status(),
        StatusCode::OK,
        "Accept failed: {:?}",
        accept_response.text().await?
    );
    let accept_body = accept_response.json::<ConnectAcceptResponse>().await?;
    assert_eq!(accept_body.status, "connected");

    // Verify both users see the connection as connected
    for user_token in [&user_a.token, &user_b.token] {
        let list_response = client
            .get(format!("{}/api/chat/connections", base_url))
            .headers(auth_headers(user_token))
            .send()
            .await?;

        assert_eq!(list_response.status(), StatusCode::OK);
        let connections = list_response.json::<ConnectionListResponse>().await?;
        let found = connections.items.iter().find(|c| c.id == connection_id);
        assert!(found.is_some(), "Connection should be visible");
        assert_eq!(found.unwrap().status, "connected");
    }

    // Step 2b: Create a second connection request to test rejection
    let request_response2 = client
        .post(format!("{}/api/chat/connect/request", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&ConnectRequestBody {
            receiver_id: user_b.user_id.clone(),
            listing_id: None,
        })
        .send()
        .await?;

    let request_body2 = request_response2.json::<ConnectRequestResponse>().await?;
    let connection_id2 = request_body2.connection_id;

    // User B rejects the second request
    let reject_response = client
        .post(format!("{}/api/chat/connect/reject", base_url))
        .headers(auth_headers(&user_b.token))
        .json(&ConnectRejectBody {
            connection_id: connection_id2.clone(),
        })
        .send()
        .await?;

    assert_eq!(reject_response.status(), StatusCode::OK);
    let reject_body = reject_response.json::<ConnectRejectResponse>().await?;
    assert_eq!(reject_body.status, "rejected");

    // Clean up
    sqlx::query("DELETE FROM chat_messages WHERE conversation_id = ANY($1)")
        .bind(&[connection_id.clone(), connection_id2.clone()])
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;
    sqlx::query("DELETE FROM chat_connections WHERE id = ANY($1)")
        .bind(&[connection_id.clone(), connection_id2])
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;

    Ok(())
}

/// Test 2: Message sending and receiving
#[tokio::test]
async fn test_message_sending() -> anyhow::Result<()> {
    let database_url = db_safety::resolve_test_database_url();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create two test users
    let user_a = create_test_user(&client, &base_url, "msg_a").await?;
    let user_b = create_test_user(&client, &base_url, "msg_b").await?;

    // Clean up
    sqlx::query(
        "DELETE FROM chat_connections WHERE (requester_id = $1 AND receiver_id = $2) OR (requester_id = $2 AND receiver_id = $1)"
    )
    .bind(&user_a.user_id)
    .bind(&user_b.user_id)
    .execute(&sqlx::PgPool::connect(&database_url).await?)
    .await?;

    // Create and accept connection
    let request_response = client
        .post(format!("{}/api/chat/connect/request", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&ConnectRequestBody {
            receiver_id: user_b.user_id.clone(),
            listing_id: None,
        })
        .send()
        .await?;

    let request_body = request_response.json::<ConnectRequestResponse>().await?;
    let connection_id = request_body.connection_id;

    client
        .post(format!("{}/api/chat/connect/accept", base_url))
        .headers(auth_headers(&user_b.token))
        .json(&ConnectAcceptBody {
            connection_id: connection_id.clone(),
        })
        .send()
        .await?;

    // Step 1: User A sends a message
    let message_content = "Hello, this is a test message!";
    let send_response = client
        .post(format!(
            "{}/api/chat/conversations/{}/messages",
            base_url, connection_id
        ))
        .headers(auth_headers(&user_a.token))
        .json(&SendMessageBody {
            content: message_content.to_string(),
            image_base64: None,
            audio_base64: None,
        })
        .send()
        .await?;

    assert_eq!(
        send_response.status(),
        StatusCode::OK,
        "Send message failed: {:?}",
        send_response.text().await?
    );
    let sent_message = send_response.json::<SendMessageResponse>().await?;
    assert_eq!(sent_message.sender, user_a.user_id);
    assert_eq!(sent_message.content, message_content);
    assert_eq!(sent_message.conversation_id, connection_id);
    assert_eq!(sent_message.status, "sent");

    // Step 2: User B receives the message via GET endpoint
    let get_response = client
        .get(format!(
            "{}/api/chat/conversations/{}/messages",
            base_url, connection_id
        ))
        .headers(auth_headers(&user_b.token))
        .send()
        .await?;

    assert_eq!(get_response.status(), StatusCode::OK);
    let messages_response = get_response.json::<MessageListResponse>().await?;
    assert!(
        !messages_response.messages.is_empty(),
        "Should have at least one message"
    );

    // Find our message (messages are returned in reverse chronological order)
    let found_message = messages_response
        .messages
        .iter()
        .find(|m| m.id == sent_message.message_id);
    assert!(found_message.is_some(), "Message should be retrievable");
    let received_msg = found_message.unwrap();
    assert_eq!(received_msg.sender, user_a.user_id);
    assert_eq!(
        received_msg.sender_username.as_deref(),
        Some(user_a.username.as_str())
    );
    assert_eq!(received_msg.content, message_content);
    assert!(
        !received_msg.timestamp.is_empty(),
        "Timestamp should be set"
    );

    // Clean up
    sqlx::query("DELETE FROM chat_messages WHERE conversation_id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;
    sqlx::query("DELETE FROM chat_connections WHERE id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;

    Ok(())
}

/// Test 3: Message editing within 15-minute window and 403 for editing other's messages
#[tokio::test]
async fn test_message_editing() -> anyhow::Result<()> {
    let database_url = db_safety::resolve_test_database_url();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create two test users
    let user_a = create_test_user(&client, &base_url, "edit_a").await?;
    let user_b = create_test_user(&client, &base_url, "edit_b").await?;

    // Clean up
    sqlx::query(
        "DELETE FROM chat_connections WHERE (requester_id = $1 AND receiver_id = $2) OR (requester_id = $2 AND receiver_id = $1)"
    )
    .bind(&user_a.user_id)
    .bind(&user_b.user_id)
    .execute(&sqlx::PgPool::connect(&database_url).await?)
    .await?;

    // Create and accept connection
    let request_response = client
        .post(format!("{}/api/chat/connect/request", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&ConnectRequestBody {
            receiver_id: user_b.user_id.clone(),
            listing_id: None,
        })
        .send()
        .await?;

    let connection_id = request_response
        .json::<ConnectRequestResponse>()
        .await?
        .connection_id;

    client
        .post(format!("{}/api/chat/connect/accept", base_url))
        .headers(auth_headers(&user_b.token))
        .json(&ConnectAcceptBody {
            connection_id: connection_id.clone(),
        })
        .send()
        .await?;

    // User A sends a message
    let send_response = client
        .post(format!(
            "{}/api/chat/conversations/{}/messages",
            base_url, connection_id
        ))
        .headers(auth_headers(&user_a.token))
        .json(&SendMessageBody {
            content: "Original message content".to_string(),
            image_base64: None,
            audio_base64: None,
        })
        .send()
        .await?;

    let sent_message = send_response.json::<SendMessageResponse>().await?;
    let message_id = sent_message.message_id;

    // Step 1: User A edits their own message within 15-minute window
    let edited_content = "Edited message content";
    let edit_response = client
        .patch(format!("{}/api/chat/messages/{}", base_url, message_id))
        .headers(auth_headers(&user_a.token))
        .json(&EditMessageBody {
            content: edited_content.to_string(),
        })
        .send()
        .await?;

    assert_eq!(
        edit_response.status(),
        StatusCode::OK,
        "Edit failed: {:?}",
        edit_response.text().await?
    );
    let edit_result = edit_response.json::<EditMessageResponse>().await?;
    assert_eq!(edit_result.content, edited_content);
    assert!(!edit_result.edited_at.is_empty(), "edited_at should be set");

    // Verify edit is reflected in subsequent GET
    let get_response = client
        .get(format!(
            "{}/api/chat/conversations/{}/messages",
            base_url, connection_id
        ))
        .headers(auth_headers(&user_a.token))
        .send()
        .await?;

    let messages = get_response.json::<MessageListResponse>().await?;
    let edited_msg = messages.messages.iter().find(|m| m.id == message_id);
    assert!(edited_msg.is_some());
    assert_eq!(edited_msg.unwrap().content, edited_content);
    assert!(
        edited_msg.unwrap().edited_at.is_some(),
        "edited_at should be set"
    );

    // Step 2: User A cannot edit User B's messages (403 Forbidden)
    // First, User B sends a message
    let b_send_response = client
        .post(format!(
            "{}/api/chat/conversations/{}/messages",
            base_url, connection_id
        ))
        .headers(auth_headers(&user_b.token))
        .json(&SendMessageBody {
            content: "Message from User B".to_string(),
            image_base64: None,
            audio_base64: None,
        })
        .send()
        .await?;

    let b_message_id = b_send_response
        .json::<SendMessageResponse>()
        .await?
        .message_id;

    // User A tries to edit User B's message
    let edit_b_response = client
        .patch(format!("{}/api/chat/messages/{}", base_url, b_message_id))
        .headers(auth_headers(&user_a.token))
        .json(&EditMessageBody {
            content: "Trying to edit User B's message".to_string(),
        })
        .send()
        .await?;

    assert_eq!(
        edit_b_response.status(),
        StatusCode::FORBIDDEN,
        "Should not allow editing other's messages"
    );

    // Clean up
    sqlx::query("DELETE FROM chat_messages WHERE conversation_id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;
    sqlx::query("DELETE FROM chat_connections WHERE id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;

    Ok(())
}

/// Test 4: Read receipts - marking messages as read
#[tokio::test]
async fn test_read_receipts() -> anyhow::Result<()> {
    let database_url = db_safety::resolve_test_database_url();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create two test users
    let user_a = create_test_user(&client, &base_url, "read_a").await?;
    let user_b = create_test_user(&client, &base_url, "read_b").await?;

    // Clean up
    sqlx::query(
        "DELETE FROM chat_connections WHERE (requester_id = $1 AND receiver_id = $2) OR (requester_id = $2 AND receiver_id = $1)"
    )
    .bind(&user_a.user_id)
    .bind(&user_b.user_id)
    .execute(&sqlx::PgPool::connect(&database_url).await?)
    .await?;

    // Create and accept connection
    let request_response = client
        .post(format!("{}/api/chat/connect/request", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&ConnectRequestBody {
            receiver_id: user_b.user_id.clone(),
            listing_id: None,
        })
        .send()
        .await?;

    let connection_id = request_response
        .json::<ConnectRequestResponse>()
        .await?
        .connection_id;

    client
        .post(format!("{}/api/chat/connect/accept", base_url))
        .headers(auth_headers(&user_b.token))
        .json(&ConnectAcceptBody {
            connection_id: connection_id.clone(),
        })
        .send()
        .await?;

    // User A sends a message
    let send_response = client
        .post(format!(
            "{}/api/chat/conversations/{}/messages",
            base_url, connection_id
        ))
        .headers(auth_headers(&user_a.token))
        .json(&SendMessageBody {
            content: "Message for read receipt test".to_string(),
            image_base64: None,
            audio_base64: None,
        })
        .send()
        .await?;

    let sent_message = send_response.json::<SendMessageResponse>().await?;
    let message_id = sent_message.message_id;

    // User B marks the message as read
    let read_response = client
        .post(format!(
            "{}/api/chat/messages/{}/read",
            base_url, message_id
        ))
        .headers(auth_headers(&user_b.token))
        .send()
        .await?;

    assert_eq!(
        read_response.status(),
        StatusCode::OK,
        "Mark read failed: {:?}",
        read_response.text().await?
    );
    let read_result = read_response.json::<MarkReadResponse>().await?;
    assert_eq!(read_result.message_id, message_id);
    assert!(!read_result.read_at.is_empty(), "read_at should be set");

    // Verify the message now has a read_at timestamp
    let get_response = client
        .get(format!(
            "{}/api/chat/conversations/{}/messages",
            base_url, connection_id
        ))
        .headers(auth_headers(&user_b.token))
        .send()
        .await?;

    let messages = get_response.json::<MessageListResponse>().await?;
    let read_msg = messages.messages.iter().find(|m| m.id == message_id);
    assert!(read_msg.is_some());
    let read_msg = read_msg.unwrap();
    assert!(
        read_msg.read_at.is_some(),
        "Message should have read_at set"
    );
    assert_eq!(read_msg.read_by.as_deref(), Some(user_b.user_id.as_str()));

    // Clean up
    sqlx::query("DELETE FROM chat_messages WHERE conversation_id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;
    sqlx::query("DELETE FROM chat_connections WHERE id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;

    Ok(())
}

/// Test 5: Typing indicator - endpoint returns 200 OK when called
#[tokio::test]
async fn test_typing_indicator() -> anyhow::Result<()> {
    let database_url = db_safety::resolve_test_database_url();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create two test users
    let user_a = create_test_user(&client, &base_url, "typing_a").await?;
    let user_b = create_test_user(&client, &base_url, "typing_b").await?;

    // Clean up
    sqlx::query(
        "DELETE FROM chat_connections WHERE (requester_id = $1 AND receiver_id = $2) OR (requester_id = $2 AND receiver_id = $1)"
    )
    .bind(&user_a.user_id)
    .bind(&user_b.user_id)
    .execute(&sqlx::PgPool::connect(&database_url).await?)
    .await?;

    // Create and accept connection
    let request_response = client
        .post(format!("{}/api/chat/connect/request", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&ConnectRequestBody {
            receiver_id: user_b.user_id.clone(),
            listing_id: None,
        })
        .send()
        .await?;

    let connection_id = request_response
        .json::<ConnectRequestResponse>()
        .await?
        .connection_id;

    client
        .post(format!("{}/api/chat/connect/accept", base_url))
        .headers(auth_headers(&user_b.token))
        .json(&ConnectAcceptBody {
            connection_id: connection_id.clone(),
        })
        .send()
        .await?;

    // User A sends typing indicator
    let typing_response = client
        .post(format!("{}/api/chat/typing", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&TypingBody {
            conversation_id: connection_id.clone(),
        })
        .send()
        .await?;

    // The typing indicator endpoint returns 200 OK on success (WS broadcast is fire-and-forget)
    assert_eq!(
        typing_response.status(),
        StatusCode::OK,
        "Typing indicator failed: {:?}",
        typing_response.text().await?
    );

    // Clean up
    sqlx::query("DELETE FROM chat_messages WHERE conversation_id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;
    sqlx::query("DELETE FROM chat_connections WHERE id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;

    Ok(())
}

/// Test 6: Cannot send messages to non-connected users (connection must be "connected")
#[tokio::test]
async fn test_cannot_send_to_non_connected() -> anyhow::Result<()> {
    let database_url = db_safety::resolve_test_database_url();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create two test users
    let user_a = create_test_user(&client, &base_url, "noconn_a").await?;
    let user_b = create_test_user(&client, &base_url, "noconn_b").await?;

    // Clean up
    sqlx::query(
        "DELETE FROM chat_connections WHERE (requester_id = $1 AND receiver_id = $2) OR (requester_id = $2 AND receiver_id = $1)"
    )
    .bind(&user_a.user_id)
    .bind(&user_b.user_id)
    .execute(&sqlx::PgPool::connect(&database_url).await?)
    .await?;

    // Create connection but DON'T accept it
    let request_response = client
        .post(format!("{}/api/chat/connect/request", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&ConnectRequestBody {
            receiver_id: user_b.user_id.clone(),
            listing_id: None,
        })
        .send()
        .await?;

    let connection_id = request_response
        .json::<ConnectRequestResponse>()
        .await?
        .connection_id;

    // Try to send a message (should fail because connection is not "connected")
    let send_response = client
        .post(format!(
            "{}/api/chat/conversations/{}/messages",
            base_url, connection_id
        ))
        .headers(auth_headers(&user_a.token))
        .json(&SendMessageBody {
            content: "This should fail".to_string(),
            image_base64: None,
            audio_base64: None,
        })
        .send()
        .await?;

    assert_eq!(
        send_response.status(),
        StatusCode::BAD_REQUEST,
        "Should not allow sending to non-connected users"
    );

    // Clean up
    sqlx::query("DELETE FROM chat_connections WHERE id = $1")
        .bind(&connection_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;

    Ok(())
}

/// Test 7: Cannot accept own connection request (must be receiver)
#[tokio::test]
async fn test_cannot_accept_own_request() -> anyhow::Result<()> {
    let database_url = db_safety::resolve_test_database_url();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create one test user
    let user_a = create_test_user(&client, &base_url, "self_accept").await?;

    // Clean up
    sqlx::query("DELETE FROM chat_connections WHERE requester_id = $1 OR receiver_id = $1")
        .bind(&user_a.user_id)
        .execute(&sqlx::PgPool::connect(&database_url).await?)
        .await?;

    // User A sends a connection request to themselves (this is prevented at request time)
    let request_response = client
        .post(format!("{}/api/chat/connect/request", base_url))
        .headers(auth_headers(&user_a.token))
        .json(&ConnectRequestBody {
            receiver_id: user_a.user_id.clone(),
            listing_id: None,
        })
        .send()
        .await?;

    // Should fail because you can't send a connection request to yourself
    assert_eq!(
        request_response.status(),
        StatusCode::BAD_REQUEST,
        "Should not allow self-connection"
    );

    Ok(())
}
