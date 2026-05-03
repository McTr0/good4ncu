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

mod models;

pub use models::{
    ConnectAcceptBody, ConnectAcceptResponse, ConnectRejectBody, ConnectRejectResponse,
    ConnectRequestBody, ConnectRequestResponse, ConnectionEntry, ConnectionListResponse,
    EditMessageBody, EditMessageResponse, MarkReadResponse, MessageEntry, MessageListQuery,
    MessageListResponse, SendMessageBody, SendMessageResponse, TypingBody,
};

mod connection;
mod context;
mod events;
mod message;

pub use connection::{connect_accept, connect_reject, connect_request, list_connections};
pub use message::{
    edit_message, get_connection_messages, mark_connection_read, mark_message_read,
    send_connection_message, typing_indicator,
};

pub(crate) use events::{
    WsConnectionEstablishedEvent, WsConnectionRejectedEvent, WsConnectionRequestEvent,
    WsMessageReadEvent, WsNewMessageEvent, WsTypingEvent,
};

#[cfg(test)]
mod tests;
