import 'dart:convert';
import '../models/models.dart';
import 'base_service.dart';

/// Chat service — handles messaging, conversations, connections, typing indicators.
class ChatService extends BaseService {
  /// Send a chat message (AI agent reply).
  /// POST /api/chat
  Future<String> sendChatMessage(ChatMessage message) async {
    final headers = await authHeaders();
    final response = await post(
      Uri.parse('$baseUrl/api/chat'),
      headers,
      jsonEncode(message.toJson()),
    );
    return handleResponse(
      response,
      (data) => data['reply'] ?? 'Empty response',
    );
  }

  /// Get all chat connections for current user.
  /// GET /api/chat/connections
  Future<List<Conversation>> getConnections() async {
    final headers = await authHeaders();
    final response = await get(
      Uri.parse('$baseUrl/api/chat/connections'),
      headers,
    );
    final data = handleResponse(response, (d) => d as Map<String, dynamic>);
    final items = data['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => Conversation.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  /// Request a new chat connection.
  /// POST /api/chat/connect/request
  Future<void> requestConnection(String receiverId, {String? listingId}) async {
    final headers = await authHeaders();
    final body = <String, dynamic>{'receiver_id': receiverId};
    if (listingId != null) body['listing_id'] = listingId;
    final response = await post(
      Uri.parse('$baseUrl/api/chat/connect/request'),
      headers,
      jsonEncode(body),
    );
    handleResponse(response, (_) {});
  }

  /// Accept a pending connection.
  /// POST /api/chat/connect/accept
  Future<void> acceptConnection(String connectionId) async {
    final headers = await authHeaders();
    final response = await post(
      Uri.parse('$baseUrl/api/chat/connect/accept'),
      headers,
      jsonEncode({'connection_id': connectionId}),
    );
    handleResponse(response, (_) {});
  }

  /// Reject a pending connection.
  /// POST /api/chat/connect/reject
  Future<void> rejectConnection(String connectionId) async {
    final headers = await authHeaders();
    final response = await post(
      Uri.parse('$baseUrl/api/chat/connect/reject'),
      headers,
      jsonEncode({'connection_id': connectionId}),
    );
    handleResponse(response, (_) {});
  }

  /// Send a private message in an established conversation.
  /// POST /api/chat/conversations/{conversationId}/messages
  Future<ConversationMessage> sendMessage(
    String conversationId, {
    required String content,
    String? imageBase64,
    String? audioBase64,
    String? imageUrl,
    String? audioUrl,
  }) async {
    final headers = await authHeaders();
    final body = <String, dynamic>{'content': content};
    if (imageBase64 != null) body['image_base64'] = imageBase64;
    if (audioBase64 != null) body['audio_base64'] = audioBase64;
    if (imageUrl != null) body['image_url'] = imageUrl;
    if (audioUrl != null) body['audio_url'] = audioUrl;

    final response = await post(
      Uri.parse('$baseUrl/api/chat/conversations/$conversationId/messages'),
      headers,
      jsonEncode(body),
    );
    return handleResponse(
      response,
      (d) => ConversationMessage.fromJson(d as Map<String, dynamic>),
    );
  }

  /// Get private chat messages for a conversation.
  /// GET /api/chat/conversations/{conversationId}/messages
  Future<List<ConversationMessage>> getChatConversationMessages(
    String conversationId, {
    int limit = 50,
    int offset = 0,
  }) async {
    final headers = await authHeaders();
    final uri = Uri.parse(
      '$baseUrl/api/chat/conversations/$conversationId/messages?limit=$limit&offset=$offset',
    );
    final response = await get(uri, headers);
    final data = handleResponse(response, (d) => d as Map<String, dynamic>);
    final messages = data['messages'] as List<dynamic>? ?? [];
    return messages
        .map((e) => ConversationMessage.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  /// Mark a single message as read.
  /// POST /api/chat/messages/{messageId}/read
  Future<void> markMessageRead(String messageId) async {
    final headers = await authHeaders();
    final response = await post(
      Uri.parse('$baseUrl/api/chat/messages/$messageId/read'),
      headers,
      '{}',
    );
    handleResponse(response, (_) {});
  }

  /// Edit a message (within 15 minutes of sending).
  /// PATCH /api/chat/messages/{id}
  Future<ConversationMessage> editMessage(
    String messageId,
    String content,
  ) async {
    final headers = await authHeaders();
    final response = await patch(
      Uri.parse('$baseUrl/api/chat/messages/$messageId'),
      headers,
      jsonEncode({'content': content}),
    );
    return handleResponse(
      response,
      (d) => ConversationMessage.fromJson(d as Map<String, dynamic>),
    );
  }

  /// Send typing indicator.
  /// POST /api/chat/typing
  Future<void> sendTyping(String conversationId) async {
    final headers = await authHeaders();
    final response = await post(
      Uri.parse('$baseUrl/api/chat/typing'),
      headers,
      jsonEncode({'conversation_id': conversationId}),
    );
    handleResponse(response, (_) {});
  }

  /// Mark entire conversation as read (batch).
  /// POST /api/chat/connection/{conversationId}/read
  Future<void> markConnectionAsRead(String conversationId) async {
    final headers = await authHeaders();
    final response = await post(
      Uri.parse('$baseUrl/api/chat/connection/$conversationId/read'),
      headers,
      '{}',
    );
    handleResponse(response, (_) {});
  }
}
