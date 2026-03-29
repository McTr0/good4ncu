# Chat System Documentation

**Last Updated:** 2026-03-27

## Overview

The chat system implements user-to-user direct messaging with a three-way handshake connection flow. It consists of:

- **HTTP REST API** for connection management and message exchange
- **WebSocket** for real-time push notifications (new messages, typing indicators, connection events)
- **PostgreSQL** for persistent message storage and connection state

---

## 1. Connection Flow (聊天连接流程)

### Three-Way Handshake

The system implements a deliberate connection handshake before allowing message exchange:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        3-WAY HANDSHAKE PROTOCOL                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  User A (Requester)                    User B (Receiver)                     │
│       │                                     │                               │
│       │── POST /api/chat/connect/request ──>│                               │
│       │     { receiver_id, listing_id? }    │                               │
│       │                                     │                               │
│       │     <───────────────────────────────│ WS: connection_request         │
│       │           (push to User B)          │                               │
│       │                                     │                               │
│       │                            +-----------------+                      │
│       │                            │ Dialog appears  │                      │
│       │                            │ [Accept] [Reject]│                     │
│       │                            +-----------------+                      │
│       │                                     │                               │
│       │<── POST /api/chat/connect/accept ──│                               │
│       │     { connection_id }               │ (or /reject)                  │
│       │                                     │                               │
│       │     = pending =                    │ = pending =                   │
│       │                                     │                               │
│       │<──────────────── WS event ─────────│                               │
│       │       connection_established        │                               │
│       │       { connection_id,              │                               │
│       │         established_at }             │                               │
│       │                                     │                               │
│       │     = connected =                   │ = connected =                 │
│       │                                     │                               │
│       │── POST /api/chat/conversations/ ─────────────────────────>│         │
│       │     { content }                     │                               │
│       │                                     │                               │
│       │<──────────────────────── WS event ─│                               │
│       │        new_message                  │                               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Connection Statuses

| Status | Description | Who Can Perform Action |
|--------|-------------|----------------------|
| `pending` | Connection requested, awaiting receiver acceptance | Receiver can accept or reject |
| `connected` | Full duplex communication established | Both parties can exchange messages |
| `rejected` | Receiver declined the connection request | Neither party can send messages; can request again |

### Rules

- Only the **receiver** (not the requester) can accept or reject a pending connection
- A requester cannot send messages until the connection is in `connected` status
- The same pair of users can have only one active connection at a time
- Re-requesting resets status to `pending` and clears `established_at`

---

## 2. WebSocket Architecture

### Endpoint

```
ws://localhost:3000/api/ws?token=<jwt>
```

**Authentication**: JWT token passed as query parameter (not header). Browsers do not send custom headers during WebSocket handshake.

### Server-Side Implementation

**File**: `/Users/mctr0/Projects/good4ncu/src/api/ws.rs`

- Uses `DashMap<String, Vec<mpsc::Sender<Message>>>` to track user connections
- One user can have **multiple simultaneous connections** (e.g., phone + tablet)
- Each connection has a dedicated mpsc channel with 64-message buffer
- Heartbeat: server sends `Ping` every 30 seconds; client responds with `Pong`
- Dead connections are pruned automatically on broadcast or heartbeat timeout

### Event Types (Server to Client)

All events are JSON with an `event` field:

```json
// Connection request received (pushed to receiver)
{
  "event": "connection_request",
  "connection_id": "uuid",
  "requester_id": "user_id",
  "requester_username": "alice",
  "listing_id": "listing_uuid"
}

// Connection established (pushed to both parties)
{
  "event": "connection_established",
  "connection_id": "uuid",
  "established_at": "2026-03-27T10:00:00Z"
}

// New message (pushed to receiver)
{
  "event": "new_message",
  "message_id": 123,
  "conversation_id": "uuid",
  "sender": "user_id",
  "sender_username": "alice",
  "content": "Hello!",
  "timestamp": "2026-03-27T10:00:00Z",
  "read_at": null,
  "image_data": null,
  "audio_data": null
}

// Message read receipt (pushed to message sender)
{
  "event": "message_read",
  "message_id": 123,
  "read_at": "2026-03-27T10:01:00Z",
  "read_by": "user_id"
}

// Typing indicator (pushed to the other party)
{
  "event": "typing",
  "conversation_id": "uuid",
  "user_id": "user_id",
  "username": "alice"
}
```

### Flutter WebSocket Service

**File**: `/Users/mctr0/Projects/good4ncu/mobile/lib/services/ws_service.dart`

```dart
class WsService {
  static const String _wsUrl = 'ws://localhost:3000/api/ws';

  /// Connect to WebSocket. Stores token for reconnect use.
  Future<void> connect() async {
    final prefs = await SharedPreferences.getInstance();
    final token = prefs.getString('jwt_token');
    _channel = WebSocketChannel.connect(
      Uri.parse('$_wsUrl?token=$token'),
    );
    _channel!.stream.listen(_handleMessage, ...);
  }

  void _handleMessage(dynamic data) {
    final json = jsonDecode(data as String);
    final eventType = json['event_type'] ?? '';

    if (eventType == 'ping') {
      _channel?.sink.add(jsonEncode({'type': 'pong'}));
      return;
    }

    final notification = WsNotification.fromJson(json);
    _controller?.add(notification);
  }
}
```

**Reconnection**: Exponential backoff (1s, 2s, 4s, ... cap at 30s)

---

## 3. API Reference

### Base URL

```
http://localhost:3000
```

### Authentication

All endpoints require JWT Bearer token in Authorization header:

```
Authorization: Bearer <jwt_token>
```

### Endpoints

#### GET /api/chat/connections

List all connections for the current user.

**Response**:
```json
{
  "items": [
    {
      "id": "connection_uuid",
      "requester_id": "user_1",
      "other_user_id": "user_2",
      "other_username": "alice",
      "status": "connected",
      "established_at": "2026-03-27T10:00:00Z",
      "created_at": "2026-03-27T09:55:00Z",
      "unread_count": 3,
      "is_receiver": false
    }
  ]
}
```

**curl**:
```bash
curl -X GET http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN"
```

---

#### POST /api/chat/connect/request

Initiate a connection request (Step 1 of handshake).

**Request Body**:
```json
{
  "receiver_id": "user_uuid",
  "listing_id": "listing_uuid"  // optional
}
```

**Response**:
```json
{
  "connection_id": "uuid",
  "status": "pending"
}
```

**Errors**:
- `400`: Cannot request connection to yourself
- `404`: Receiver not found

**curl**:
```bash
curl -X POST http://localhost:3000/api/chat/connect/request \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"receiver_id": "user_uuid", "listing_id": "listing_uuid"}'
```

---

#### POST /api/chat/connect/accept

Accept a pending connection request (Step 2 of handshake).

**Request Body**:
```json
{
  "connection_id": "uuid"
}
```

**Response**:
```json
{
  "status": "connected",
  "established_at": "2026-03-27T10:00:00Z"
}
```

**Errors**:
- `403`: Only the receiver can accept
- `400`: Connection is not in `pending` status

**curl**:
```bash
curl -X POST http://localhost:3000/api/chat/connect/accept \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"connection_id": "connection_uuid"}'
```

---

#### POST /api/chat/connect/reject

Reject a pending connection request.

**Request Body**:
```json
{
  "connection_id": "uuid"
}
```

**Response**:
```json
{
  "status": "rejected"
}
```

**Errors**:
- `403`: Only the receiver can reject
- `400`: Connection is not in `pending` status

**curl**:
```bash
curl -X POST http://localhost:3000/api/chat/connect/reject \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"connection_id": "connection_uuid"}'
```

---

#### GET /api/chat/conversations/{id}/messages

Fetch messages in a connection.

**Query Parameters**:
- `limit` (optional): Number of messages (default 50, max 200)
- `offset` (optional): Pagination offset (default 0)

**Response**:
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
      "timestamp": "2026-03-27T10:00:00Z",
      "read_at": "2026-03-27T10:01:00Z",
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

**curl**:
```bash
curl -X GET "http://localhost:3000/api/chat/conversations/conn_uuid/messages?limit=50&offset=0" \
  -H "Authorization: Bearer $TOKEN"
```

---

#### POST /api/chat/conversations/{id}/messages

Send a message in a connected conversation.

**Request Body**:
```json
{
  "content": "Hello!",
  "image_base64": "optional_base64_image",
  "audio_base64": "optional_base64_audio"
}
```

**Response**:
```json
{
  "id": 123,
  "sender": "user_id",
  "content": "Hello!",
  "conversation_id": "uuid",
  "timestamp": "2026-03-27T10:00:00Z",
  "read_at": "2026-03-27T10:00:00Z",
  "image_data": null,
  "audio_data": null,
  "status": "sent"
}
```

**Validation**:
- Content must be 1-2000 characters
- Connection must be in `connected` status

**curl**:
```bash
curl -X POST http://localhost:3000/api/chat/conversations/conn_uuid/messages \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content": "Hello!"}'
```

---

#### POST /api/chat/messages/{id}/read

Mark a single message as read.

**curl**:
```bash
curl -X POST http://localhost:3000/api/chat/messages/123/read \
  -H "Authorization: Bearer $TOKEN"
```

---

#### POST /api/chat/connection/{id}/read

Batch mark all messages in a connection as read (resets `unread_count`).

**curl**:
```bash
curl -X POST http://localhost:3000/api/chat/connection/conn_uuid/read \
  -H "Authorization: Bearer $TOKEN"
```

---

#### PATCH /api/chat/messages/{id}

Edit a message within 15 minutes of sending.

**Request Body**:
```json
{
  "content": "Updated message"
}
```

**Errors**:
- `400`: Message is older than 15 minutes

**curl**:
```bash
curl -X PATCH http://localhost:3000/api/chat/messages/123 \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content": "Updated message"}'
```

---

#### POST /api/chat/typing

Send a typing indicator to the other party.

**Request Body**:
```json
{
  "conversation_id": "uuid"
}
```

**curl**:
```bash
curl -X POST http://localhost:3000/api/chat/typing \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"conversation_id": "conn_uuid"}'
```

---

## 4. Known Issues

### CORS Configuration Required

**Problem**: WebSocket connections or API requests fail with CORS errors in production.

**Solution**: Set the `CORS_ORIGINS` environment variable to specify allowed origins:

```bash
# Allow specific origins (comma-separated)
CORS_ORIGINS=https://example.com,https://app.example.com

# Or use wildcard for development only
CORS_ORIGINS=*
```

**Note**: In production, always set `CORS_ORIGINS` to specific origins. Empty `CORS_ORIGINS` defaults to allowing all origins (development mode).

**File**: `/Users/mctr0/Projects/good4ncu/src/api/mod.rs` (lines 182-206)

---

### Rate Limiting on WebSocket

**Problem**: Rate limiting middleware may block WebSocket connections or cause intermittent failures.

**Solution**: The `/api/ws` endpoint is **whitelisted** from rate limiting.

**Whitelisted paths** (from `/Users/mctr0/Projects/good4ncu/src/middleware/rate_limit/local.rs`):
```rust
const WHITELISTED_PATHS: &[&str] = &[
    "/api/health",
    "/api/stats",
    "/api/categories",
    "/api/chat/connections",
    "/api/chat/conversations",
    "/api/chat/messages",
    "/api/ws",
];
```

---

### Buyer Offline When Seller Accepts

**Problem**: When the seller accepts a connection request, if the buyer is offline, they will not receive the `connection_established` WebSocket event immediately.

**Impact**: The buyer must refresh or reconnect to see the established connection.

**Current Behavior**: WebSocket push is fire-and-forget; if the client is disconnected, the event is lost.

**Workaround**: The Flutter app calls `_loadConnectionStatus()` on page load to sync state from the API.

---

## 5. Flutter State Management

### Connection Status State

**File**: `/Users/mctr0/Projects/good4ncu/mobile/lib/pages/user_chat_page.dart`

```dart
class _UserChatPageState extends State<UserChatPage> {
  /// 连接状态: null=无连接, 'connecting'=连接中, 'connected'=已连接
  String? _connectionStatus;

  /// 对方正在输入状态
  bool _isOtherTyping = false;
}
```

### Initial Sync on Page Load

```dart
/// 从 API 加载当前连接状态，避免打开页面时不知道连接已建立
Future<void> _loadConnectionStatus() async {
  try {
    final connections = await _apiService.getConnections();
    if (!mounted) return;
    final conn = connections
        .where((c) => c.id == widget.conversationId)
        .firstOrNull;
    if (conn != null && conn.status != 'pending') {
      setState(() => _connectionStatus = 'connected');
    }
  } catch (e) {
    debugPrint('_loadConnectionStatus error: $e');
  }
}
```

### WebSocket Event Handling

```dart
void _handleWsNotification(WsNotification notif) {
  if (!mounted) return;

  switch (notif.eventType) {
    case 'connection_established':
      setState(() => _connectionStatus = 'connected');
      _loadMessages();
      _showSnackBar('连接已建立');
      break;

    case 'new_message':
      final messageId = notif.messageId;
      if (messageId != null) {
        _apiService.markMessageRead(messageId).catchError((_) {});
        _loadMessages();
      }
      break;

    case 'message_read':
      _loadMessages();
      break;

    case 'typing':
      if (notif.conversationId == widget.conversationId &&
          notif.typingUserId != _currentUserId) {
        setState(() => _isOtherTyping = true);
        _typingTimer?.cancel();
        _typingTimer = Timer(const Duration(seconds: 3), () {
          if (mounted) setState(() => _isOtherTyping = false);
        });
      }
      break;

    case 'connection_request':
      _showConnectionRequestDialog(notif);
      break;
  }
}
```

### Connection Request Dialog

When a `connection_request` event is received, the Flutter app shows a dialog:

```dart
void _showConnectionRequestDialog(WsNotification notif) {
  final connectionId = notif.connectionId;
  if (connectionId == null) return;

  showDialog(
    context: context,
    barrierDismissible: false,
    builder: (ctx) => AlertDialog(
      title: const Text('连接请求'),
      content: Text('${notif.title}\n\n${notif.body}\n\n确认后将开启消息已读功能'),
      actions: [
        TextButton(
          onPressed: () {
            Navigator.pop(ctx);
            _rejectConnection(connectionId);
          },
          child: const Text('拒绝'),
        ),
        ElevatedButton(
          onPressed: () {
            Navigator.pop(ctx);
            _acceptConnection(connectionId);
          },
          child: const Text('接受'),
        ),
      ],
    ),
  );
}
```

### UI Indicator

The connection status is displayed in the app bar using `_ConnectionIndicator`:

| Status | Color | Label |
|--------|-------|-------|
| `connected` | Green | 在线 |
| `connecting` | Yellow (animated) | 连接中... |
| `null`/other | Grey | 离线 |

---

## Database Schema

### chat_connections

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `requester_id` | TEXT | User who initiated the request |
| `receiver_id` | TEXT | User who received the request |
| `status` | TEXT | `pending`, `connected`, or `rejected` |
| `established_at` | TIMESTAMPTZ | When connection was established |
| `created_at` | TIMESTAMPTZ | When request was created |
| `unread_count` | INTEGER | Unread message count for badge |

**Unique constraint**: `(requester_id, receiver_id)`

### chat_messages

| Column | Type | Description |
|--------|------|-------------|
| `id` | BIGSERIAL | Primary key |
| `conversation_id` | TEXT | References `chat_connections.id` |
| `listing_id` | TEXT | Always `'direct'` for user chat |
| `sender` | TEXT | Sender user ID |
| `receiver` | TEXT | Receiver user ID (nullable for agent messages) |
| `is_agent` | BOOLEAN | True if message is from AI agent |
| `content` | TEXT | Message text |
| `image_data` | TEXT | Base64 encoded image (optional) |
| `audio_data` | TEXT | Base64 encoded audio (optional) |
| `read_at` | TIMESTAMPTZ | When message was read |
| `read_by` | TEXT | User ID who read the message |
| `timestamp` | TIMESTAMPTZ | When message was sent |
| `edited_at` | TIMESTAMPTZ | When message was last edited |
| `status` | TEXT | `sending`, `sent`, `delivered`, `read`, or `failed` |

---

## Related Files

| File | Purpose |
|------|---------|
| `/Users/mctr0/Projects/good4ncu/src/api/user_chat.rs` | Main chat API handlers |
| `/Users/mctr0/Projects/good4ncu/src/api/ws.rs` | WebSocket implementation |
| `/Users/mctr0/Projects/good4ncu/src/api/mod.rs` | Router and AppState setup |
| `/Users/mctr0/Projects/good4ncu/src/middleware/rate_limit/local.rs` | Rate limiting with whitelist |
| `/Users/mctr0/Projects/good4ncu/mobile/lib/services/ws_service.dart` | Flutter WebSocket client |
| `/Users/mctr0/Projects/good4ncu/mobile/lib/pages/user_chat_page.dart` | Flutter chat UI |
| `/Users/mctr0/Projects/good4ncu/mobile/lib/services/api_service.dart` | Flutter API client |
| `/Users/mctr0/Projects/good4ncu/migrations/0001__init.sql` | Database schema |
| `/Users/mctr0/Projects/good4ncu/migrations/0004_chat_message_status.sql` | Status field additions |
