# Chat Connection System Test Plan

## Overview

Tests cover the three-way handshake for user-to-user chat connections:
1. **Request**: `POST /api/chat/connect/request` - User A requests connection with User B
2. **Accept/Reject**: `POST /api/chat/connect/accept` or `/reject` - User B accepts/rejects
3. **Message**: `POST /api/chat/conversations/{id}/messages` - Send messages after connected

## Prerequisites

```bash
# Server must be running on localhost:3000
cargo build && cargo run

# Environment variables required
export DATABASE_URL="postgres://..."
export JWT_SECRET="your-secret-key"
export GEMINI_API_KEY="your-api-key"
```

---

## Backend API Tests (curl)

### Test 1: Register 2 Users and Get Tokens

**Setup**: Create two test users and capture their JWT tokens.

```bash
# Register User A
curl -s -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"usera","password":"Test123!","phone":"13800000001"}' | jq .

# Expected response:
# {
#   "token": "eyJhbGciOiJIUzI1NiJ9...",
#   "user": {
#     "id": "uuid-of-usera",
#     "username": "usera"
#   }
# }

# Save token for User A
TOKEN_A=$(curl -s -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"usera","password":"Test123!","phone":"13800000001"}' | jq -r '.token')

# Register User B
TOKEN_B=$(curl -s -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"userb","password":"Test123!","phone":"13800000002"}' | jq -r '.token')

# Get User B's ID
USER_B_ID=$(curl -s http://localhost:3000/api/users/search?q=userb \
  -H "Authorization: Bearer $TOKEN_B" | jq -r '.[0].id')
```

### Test 2: User A Sends Connection Request to User B

**Objective**: Verify both users see the pending connection with correct `is_receiver` flag.

```bash
# User A sends connection request to User B
curl -s -X POST http://localhost:3000/api/chat/connect/request \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_A" \
  -d "{\"receiver_id\":\"$USER_B_ID\"}" | jq .

# Expected response:
# {
#   "connection_id": "uuid-xxxxx-xxxx",
#   "status": "pending"
# }

# Save connection_id
CONNECTION_ID=$(curl -s -X POST http://localhost:3000/api/chat/connect/request \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_A" \
  -d "{\"receiver_id\":\"$USER_B_ID\"}" | jq -r '.connection_id')

# Verify User A sees the connection (is_receiver = false)
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN_A" | jq .

# Expected items[0]:
# {
#   "id": "uuid-xxxxx-xxxx",
#   "requester_id": "uuid-of-usera",
#   "other_user_id": "uuid-of-userb",
#   "status": "pending",
#   "is_receiver": false,
#   ...
# }

# Verify User B sees the connection (is_receiver = true)
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN_B" | jq .

# Expected items[0]:
# {
#   "id": "uuid-xxxxx-xxxx",
#   "requester_id": "uuid-of-usera",
#   "other_user_id": "uuid-of-usera",
#   "status": "pending",
#   "is_receiver": true,
#   ...
# }
```

### Test 3: User B Accepts the Connection

**Objective**: Verify status changes to `connected` for both users.

```bash
# User B accepts the connection
curl -s -X POST http://localhost:3000/api/chat/connect/accept \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_B" \
  -d "{\"connection_id\":\"$CONNECTION_ID\"}" | jq .

# Expected response:
# {
#   "status": "connected",
#   "established_at": "2026-03-27T12:00:00Z"
# }

# Verify User A sees connected status
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN_A" | jq -r '.items[0].status'
# Expected: "connected"

# Verify User B sees connected status
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN_B" | jq -r '.items[0].status'
# Expected: "connected"
```

### Test 4: User A Tries to Accept (Should Get 403 Forbidden)

**Objective**: Only the receiver can accept a connection request.

```bash
# User A tries to accept (should fail - they are the requester, not receiver)
curl -s -X POST http://localhost:3000/api/chat/connect/accept \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_A" \
  -d "{\"connection_id\":\"$CONNECTION_ID\"}" | jq .

# Expected response:
# HTTP 403 Forbidden
# {
#   "error": "您没有权限执行此操作"
# }
```

### Test 5: User B Rejects a NEW Pending Connection

**Objective**: Create a new connection and verify rejection works.

```bash
# User A creates a new connection request
NEW_CONNECTION=$(curl -s -X POST http://localhost:3000/api/chat/connect/request \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_A" \
  -d "{\"receiver_id\":\"$USER_B_ID\"}" | jq -r '.connection_id')

# User B rejects the new connection
curl -s -X POST http://localhost:3000/api/chat/connect/reject \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_B" \
  -d "{\"connection_id\":\"$NEW_CONNECTION\"}" | jq .

# Expected response:
# {
#   "status": "rejected"
# }

# Verify status is rejected
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN_A" | jq -r '.items[] | select(.id == "'$NEW_CONNECTION'") | .status'
# Expected: "rejected"
```

### Test 6: User Sends Message After Connected

**Objective**: Verify messages are persisted and accessible by both parties.

```bash
# User A sends a message to the connected conversation
curl -s -X POST "http://localhost:3000/api/chat/conversations/$CONNECTION_ID/messages" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_A" \
  -d '{"content":"Hello, is this still available?"}' | jq .

# Expected response:
# {
#   "id": 1,
#   "sender": "uuid-of-usera",
#   "content": "Hello, is this still available?",
#   "conversation_id": "uuid-xxxxx-xxxx",
#   "sent_at": "2026-03-27T12:00:00Z",
#   "status": "sent",
#   ...
# }

# User B fetches messages
curl -s "http://localhost:3000/api/chat/conversations/$CONNECTION_ID/messages" \
  -H "Authorization: Bearer $TOKEN_B" | jq .

# Expected: messages array contains the sent message
```

### Test 7: Invalid Connection ID Returns 404

```bash
# Try to accept with non-existent connection_id
curl -s -X POST http://localhost:3000/api/chat/connect/accept \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_B" \
  -d '{"connection_id":"00000000-0000-0000-0000-000000000000"}' | jq .

# Expected response:
# HTTP 404 Not Found
# {
#   "error": "资源不存在"
# }
```

### Test 8: Expired/Invalid JWT Returns 401

```bash
# Try to list connections with invalid token
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer invalid-token-here" | jq .

# Expected response:
# HTTP 401 Unauthorized
# {
#   "error": "请先登录后再操作"
# }

# Try with malformed token (missing signature part)
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9" | jq .

# Expected: HTTP 401 Unauthorized
```

---

## WebSocket Tests

### Test 9: WebSocket Connects with Valid Token

```bash
# Using websocat or wscat
# Install: cargo install websocat

# Connect WebSocket with valid token
websocat "ws://localhost:3000/api/ws?token=$TOKEN_A"

# Expected: Connection succeeds, no error response
# Server accepts upgrade and registers connection

# To verify, send a message from User B and observe WebSocket push
# User B accepts a new connection request, then:
curl -s -X POST "http://localhost:3000/api/chat/connect/request" \
  -H "Authorization: Bearer $TOKEN_B" \
  -H "Content-Type: application/json" \
  -d "{\"receiver_id\":\"$(curl -s http://localhost:3000/api/users/search?q=usera -H 'Authorization: Bearer '$TOKEN_B'' | jq -r '.[0].id')\"}" | jq -r '.connection_id' > /tmp/new_conn.txt

# User A should receive connection_request event via WebSocket
```

### Test 10: WebSocket Connects with Invalid Token

```bash
# Try to connect with invalid token
websocat "ws://localhost:3000/api/ws?token=invalid-token" 2>&1 || true

# Expected: Connection rejected with 401
# HTTP/1.1 401 Unauthorized
# {"error": "Invalid or missing token"}
```

### Test 11: After Accept, Verify connection_established Event

**Prerequisite**: Connected conversation exists.

```bash
# Terminal 1: Connect User A's WebSocket
websocat "ws://localhost:3000/api/ws?token=$TOKEN_A"

# Terminal 2: User B accepts the connection
curl -s -X POST http://localhost:3000/api/chat/connect/accept \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN_B" \
  -d "{\"connection_id\":\"$CONNECTION_ID\"}"

# Terminal 1 Expected WebSocket message received:
# {
#   "event": "connection_established",
#   "connection_id": "uuid-xxxxx-xxxx",
#   "established_at": "2026-03-27T12:00:00Z"
# }
```

---

## Flutter Mobile Tests

### Test 12: Login as User A - Pending Connection Shows Accept/Reject

**Steps**:
1. Launch Flutter app
2. Login as `usera`
3. Navigate to conversation list

**Expected**:
- Pending connection to User B appears
- Connection entry shows `status: "pending"`
- UI displays **Accept** and **Reject** buttons
- `is_receiver: true` for this connection entry

**Verification**:
```bash
# API verification
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN_A" | jq '.items[] | {status, is_receiver}'
```

### Test 13: Login as User B - Connected Shows Message Input

**Steps**:
1. Logout from User A
2. Login as `userb`
3. Navigate to the conversation with User A

**Expected**:
- Connection shows `status: "connected"`
- Message input field is visible
- No Accept/Reject buttons (connection already established)

### Test 14: Pending Connection Shows "等待对方接受连接"

**Steps**:
1. Create a new pending connection (User A -> User B)
2. On User B's device, navigate to that conversation page

**Expected UI**:
- Banner or placeholder text: "等待对方接受连接" (Waiting for the other party to accept)
- Message input should be hidden or disabled
- No message sending capability until accepted

### Test 15: After Accept, Exit and Re-enter Chat

**Steps**:
1. User B accepts the connection
2. User B navigates back to conversation list
3. User B taps on the conversation again

**Expected**:
- Conversation now shows message input (connected state)
- No "waiting" message displayed
- Full chat functionality available

---

## Edge Cases

### Test 16: Rapidly Tap Accept Multiple Times

**Setup**: Pending connection exists.

**Steps**:
1. On receiver's device, rapidly tap Accept button 5+ times quickly

**Expected**:
- Only ONE API call is made (debounced client-side)
- Or: API returns success for first call, subsequent calls return appropriate error
- UI state remains consistent

**Verification**:
```bash
# Check server-side: connection should only be accepted once
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN_B" | jq '.items[] | select(.other_user_id == "uuid-of-usera") | .status'
# Expected: "connected" (not an error)

# If backend has idempotency, second accept should return 400 Bad Request
# (connection status is no longer "pending")
```

### Test 17: Accept Succeeds but WebSocket Disconnected

**Steps**:
1. User B's WebSocket disconnects (network issue)
2. User A accepts a connection with User B
3. User B's WebSocket reconnects

**Expected**:
- Accept API returns `200 OK`
- After reconnect, User B receives `connection_established` event via polling or reconnect
- Conversation list shows connected status on next refresh

**Verification**:
```bash
# User B's WebSocket disconnected, then:
curl -s -X POST http://localhost:3000/api/chat/connect/accept \
  -H "Authorization: Bearer $TOKEN_B" \
  -H "Content-Type: application/json" \
  -d "{\"connection_id\":\"$CONNECTION_ID\"}"
# Returns: {"status": "connected", "established_at": "..."}

# User B reconnects WebSocket and fetches connections
curl -s http://localhost:3000/api/chat/connections \
  -H "Authorization: Bearer $TOKEN_B" | jq '.items[0].status'
# Expected: "connected"
```

### Test 18: Two Browsers, Same User

**Setup**: User A is logged in on two different browsers/devices.

**Steps**:
1. Connect WebSocket on Browser 1 (Desktop)
2. Connect WebSocket on Browser 2 (Mobile)
3. User B accepts the connection

**Expected**:
- Both Browser 1 and Browser 2 receive `connection_established` event
- Both show connected status
- Either device can send/receive messages
- `WS_CONNECTIONS` map has two entries for User A's user_id

**Verification**:
```bash
# Send a message from User B
curl -s -X POST "http://localhost:3000/api/chat/conversations/$CONNECTION_ID/messages" \
  -H "Authorization: Bearer $TOKEN_B" \
  -H "Content-Type: application/json" \
  -d '{"content":"Message to both devices"}'

# Both Browser 1 and Browser 2 should receive new_message event
```

---

## Test Data Cleanup

After all tests complete, clean up test data:

```bash
# Option 1: Database truncation (if test DB)
curl -s -X POST http://localhost:3000/api/admin/orders \
  # ... use admin endpoint or direct DB access

# Option 2: Delete test users via admin or directly
# TRUNCATE TABLE chat_messages, chat_connections, users CASCADE;
```

---

## Test Execution Checklist

- [ ] Server running on `localhost:3000`
- [ ] Valid `DATABASE_URL`, `JWT_SECRET`, `GEMINI_API_KEY` in environment
- [ ] `websocat` or `wscat` installed for WebSocket testing
- [ ] `jq` installed for JSON parsing
- [ ] Two test user accounts available
- [ ] Flutter app built and ready for mobile tests

---

## Expected Test Results Summary

| Test | Description | Expected Result |
|------|-------------|-----------------|
| 1 | Register 2 users | 200 OK, tokens returned |
| 2 | Send connect request | 200 OK, both see pending, is_receiver correct |
| 3 | Accept connection | 200 OK, status = connected |
| 4 | Non-receiver accepts | 403 Forbidden |
| 5 | Reject connection | 200 OK, status = rejected |
| 6 | Send message | 200 OK, both can fetch |
| 7 | Invalid connection_id | 404 Not Found |
| 8 | Invalid JWT | 401 Unauthorized |
| 9 | Valid WS connect | Upgrade succeeds |
| 10 | Invalid WS token | 401 Unauthorized |
| 11 | connection_established event | Event received via WS |
| 12 | User A pending UI | Accept/Reject buttons shown |
| 13 | User B connected UI | Message input shown |
| 14 | Pending state UI | "等待对方接受连接" shown |
| 15 | Re-enter connected | Message input shown |
| 16 | Rapid accept taps | Only one API call |
| 17 | Accept + disconnected WS | API success, state correct on reconnect |
| 18 | Two browsers same user | Both receive events |
