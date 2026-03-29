# User Chat API

**Last Updated:** 2026-03-28

User-to-user direct messaging API with a three-way handshake connection flow.

## Base URL

```
http://localhost:3000
```

## Authentication

All endpoints require JWT Bearer token in Authorization header:

```
Authorization: Bearer <jwt_token>
```

## Connection Flow

The system implements a deliberate connection handshake before allowing message exchange:

1. **Request**: `POST /api/chat/connect/request` — initiate connection
2. **Accept/Reject**: `POST /api/chat/connect/accept` or `/reject` — receiver decides
3. **Message**: `POST /api/chat/conversations/{id}/messages` — exchange messages

---

## Endpoints

### POST /api/chat/connect/request

Initiate a connection request (step 1 of 3-way handshake).

**Auth**: Required

**Request Body**:
```json
{
  "receiver_id": "user_uuid",
  "listing_id": "listing_uuid"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `receiver_id` | string | Yes | UUID of user to connect with |
| `listing_id` | string | No | Listing context for the connection |

**Response** (201 Created):
```json
{
  "connection_id": "uuid",
  "status": "pending"
}
```

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 400 | `{"error": "不能向自己发起连接"}` | Cannot connect to yourself |
| 401 | `{"error": "请先登录后再操作"}` | Unauthorized |
| 404 | `{"error": "资源不存在"}` | Receiver not found |

**WebSocket Event**: Pushes `connection_request` to receiver.

---

### POST /api/chat/connect/accept

Accept a pending connection request (step 2 of handshake).

**Auth**: Required (must be receiver)

**Request Body**:
```json
{
  "connection_id": "uuid"
}
```

**Response** (200 OK):
```json
{
  "status": "connected",
  "established_at": "2026-03-28T10:00:00Z"
}
```

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 400 | `{"error": "连接状态不是 pending，当前状态: {status}"}` | Connection not pending |
| 403 | `{"error": "您没有权限执行此操作"}` | Not the receiver |
| 404 | `{"error": "资源不存在"}` | Connection not found |

**WebSocket Event**: Pushes `connection_established` to both parties.

---

### POST /api/chat/connect/reject

Reject a pending connection request.

**Auth**: Required (must be receiver)

**Request Body**:
```json
{
  "connection_id": "uuid"
}
```

**Response** (200 OK):
```json
{
  "status": "rejected"
}
```

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 400 | `{"error": "连接状态不是 pending，当前状态: {status}"}` | Connection not pending |
| 403 | `{"error": "您没有权限执行此操作"}` | Not the receiver |
| 404 | `{"error": "资源不存在"}` | Connection not found |

---

### GET /api/chat/connections

List all connections for the current user.

**Auth**: Required

**Response** (200 OK):
```json
{
  "items": [
    {
      "id": "connection_uuid",
      "requester_id": "user_1",
      "other_user_id": "user_2",
      "other_username": "alice",
      "status": "connected",
      "established_at": "2026-03-28T10:00:00Z",
      "created_at": "2026-03-28T09:55:00Z",
      "unread_count": 3,
      "is_receiver": false
    }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `status` | string | `pending`, `connected`, or `rejected` |
| `unread_count` | integer | Number of unread messages |
| `is_receiver` | boolean | Whether current user is the receiver |

---

### GET /api/chat/conversations/{id}/messages

Fetch messages in a connection.

**Auth**: Required

**Path Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Connection UUID |

**Query Parameters**:
| Parameter | Type | Default | Max | Description |
|-----------|------|---------|-----|-------------|
| `limit` | integer | 50 | 200 | Number of messages |
| `offset` | integer | 0 | - | Pagination offset |

**Response** (200 OK):
```json
{
  "conversation_id": "uuid",
  "messages": [
    {
      "id": 123,
      "sender": "user_id",
      "sender_username": "alice",
      "content": "Hello!",
      "is_agent": false,
      "timestamp": "2026-03-28T10:00:00Z",
      "read_at": "2026-03-28T10:01:00Z",
      "read_by": "user_id",
      "image_data": null,
      "audio_data": null,
      "status": "sent",
      "edited_at": null
    }
  ],
  "total": 50
}
```

| Field | Type | Description |
|-------|------|-------------|
| `is_agent` | boolean | True if message is from AI assistant |
| `status` | string | `sending`, `sent`, `delivered`, `read`, or `failed` |
| `edited_at` | string? | RFC3339 timestamp if edited |

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 403 | `{"error": "您没有权限执行此操作"}` | Not part of connection |
| 404 | `{"error": "资源不存在"}` | Connection not found |

---

### POST /api/chat/conversations/{id}/messages

Send a message in a connected conversation.

**Auth**: Required

**Path Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Connection UUID |

**Request Body**:
```json
{
  "content": "Hello!",
  "image_base64": "optional_base64_image",
  "audio_base64": "optional_base64_audio"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `content` | string | Yes | Message text (1-2000 chars) |
| `image_base64` | string | No | Base64 encoded image |
| `audio_base64` | string | No | Base64 encoded audio |

**Response** (201 Created):
```json
{
  "id": 123,
  "sender": "user_id",
  "content": "Hello!",
  "conversation_id": "uuid",
  "timestamp": "2026-03-28T10:00:00Z",
  "read_at": "2026-03-28T10:00:00Z",
  "image_data": null,
  "audio_data": null,
  "status": "sent"
}
```

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 400 | `{"error": "消息内容不能为空"}` | Empty content |
| 400 | `{"error": "消息内容不能超过2000字符"}` | Content too long |
| 400 | `{"error": "连接状态不是 connected，当前状态: {status}"}` | Connection not established |
| 403 | `{"error": "您没有权限执行此操作"}` | Not part of connection |
| 404 | `{"error": "资源不存在"}` | Connection not found |

**WebSocket Event**: Pushes `new_message` to the other party.

---

### POST /api/chat/messages/{id}/read

Mark a single message as read.

**Auth**: Required (must be receiver of the message)

**Path Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | integer | Message ID |

**Response** (200 OK):
```json
{
  "message_id": 123,
  "read_at": "2026-03-28T10:01:00Z"
}
```

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 400 | `{"error": "连接状态不是 connected，当前状态: {status}"}` | Connection not established |
| 403 | `{"error": "您没有权限执行此操作"}` | Not the receiver |
| 404 | `{"error": "资源不存在"}` | Message not found |

**WebSocket Event**: Pushes `message_read` to the message sender.

---

### POST /api/chat/connection/{id}/read

Batch mark all messages in a connection as read.

**Auth**: Required (must be part of connection)

**Path Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | string | Connection UUID |

**Response** (200 OK):
```json
{
  "marked_count": 5,
  "read_at": "2026-03-28T10:01:00Z"
}
```

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 403 | `{"error": "您没有权限执行此操作"}` | Not part of connection |
| 404 | `{"error": "资源不存在"}` | Connection not found |

---

### PATCH /api/chat/messages/{id}

Edit a message within 15 minutes of sending.

**Auth**: Required (must be sender)

**Path Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `id` | integer | Message ID |

**Request Body**:
```json
{
  "content": "Updated message"
}
```

**Response** (200 OK):
```json
{
  "message_id": 123,
  "content": "Updated message",
  "edited_at": "2026-03-28T10:05:00Z"
}
```

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 400 | `{"error": "消息内容不能为空"}` | Empty content |
| 400 | `{"error": "消息内容不能超过2000字符"}` | Content too long |
| 400 | `{"error": "消息已超过15分钟，无法编辑"}` | Edit window expired |
| 400 | `{"error": "连接状态不是 connected，当前状态: {status}"}` | Connection not established |
| 403 | `{"error": "您没有权限执行此操作"}` | Not the sender |
| 404 | `{"error": "资源不存在"}` | Message not found |

---

### POST /api/chat/typing

Send a typing indicator to the other party.

**Auth**: Required

**Request Body**:
```json
{
  "conversation_id": "uuid"
}
```

**Response** (200 OK): Empty body

**Errors**:
| Status | Body | Description |
|--------|------|-------------|
| 403 | `{"error": "您没有权限执行此操作"}` | Not part of connection |
| 404 | `{"error": "资源不存在"}` | Connection not found |

**WebSocket Event**: Pushes `typing` to the other party.

---

## WebSocket Events

Clients connect via `GET /api/ws?token=<jwt>`.

### Event Types

| Event | Direction | Description |
|-------|-----------|-------------|
| `connection_request` | Server to Receiver | New connection request received |
| `connection_established` | Server to Both | Connection accepted and established |
| `new_message` | Server to Receiver | New direct message received |
| `message_read` | Server to Sender | Message was marked as read |
| `typing` | Server to Other Party | User is typing |

### Event Payloads

```json
// connection_request
{
  "event": "connection_request",
  "connection_id": "uuid",
  "requester_id": "user_id",
  "requester_username": "alice",
  "listing_id": "listing_uuid"
}

// connection_established
{
  "event": "connection_established",
  "connection_id": "uuid",
  "established_at": "2026-03-28T10:00:00Z"
}

// new_message
{
  "event": "new_message",
  "message_id": 123,
  "conversation_id": "uuid",
  "sender": "user_id",
  "sender_username": "alice",
  "content": "Hello!",
  "timestamp": "2026-03-28T10:00:00Z",
  "read_at": null,
  "image_data": null,
  "audio_data": null
}

// message_read
{
  "event": "message_read",
  "message_id": 123,
  "read_at": "2026-03-28T10:01:00Z",
  "read_by": "user_id"
}

// typing
{
  "event": "typing",
  "conversation_id": "uuid",
  "user_id": "user_id",
  "username": "alice"
}
```

---

## Connection Statuses

| Status | Description | Allowed Actions |
|--------|-------------|-----------------|
| `pending` | Awaiting receiver acceptance | Receiver can accept or reject |
| `connected` | Full duplex communication | Both parties can exchange messages |
| `rejected` | Receiver declined | Can request again (resets to pending) |

---

## Rules

- Only the **receiver** can accept or reject a pending connection
- A requester cannot send messages until connection is `connected`
- One active connection per user pair
- Re-requesting resets status to `pending` and clears `established_at`
- Messages can be edited within 15 minutes of sending
- Rate limiting: `/api/chat` endpoint is rate-limited (20 req/min per IP)
- Whitelisted from rate limiting: health, stats, categories, chat read endpoints, WebSocket

---

## Related Files

| File | Purpose |
|------|---------|
| `src/api/user_chat.rs` | Main chat API handlers |
| `src/api/ws.rs` | WebSocket implementation |
| `src/api/mod.rs` | Router and AppState setup |
| `src/api/error.rs` | ApiError enum with HTTP status mappings |
| `src/services/chat.rs` | ChatService business logic |
| `docs/chat_system.md` | Full chat system documentation |
