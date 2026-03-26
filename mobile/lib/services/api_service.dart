import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import '../models/models.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Thrown when API returns 401 Unauthorized
class AuthException implements Exception {
  final String message;
  AuthException([this.message = 'Session expired. Please login again.']);
  @override
  String toString() => message;
}

/// Thrown when API returns 409 Conflict
class ConflictException implements Exception {
  final String message;
  ConflictException([this.message = 'Resource conflict.']);
  @override
  String toString() => message;
}

/// 网络不可达 / 超时
class NetworkException implements Exception {
  final String message;
  NetworkException([this.message = '网络连接失败，请检查网络设置']);
  @override
  String toString() => message;
}

/// 服务器错误（5xx）
class ServerException implements Exception {
  final String message;
  final int statusCode;
  ServerException(this.statusCode, [this.message = '服务器异常，请稍后再试']);
  @override
  String toString() => message;
}

class ApiService {
  // Global navigator key for programmatic navigation (e.g., force logout)
  static final GlobalKey<NavigatorState> navigatorKey = GlobalKey<NavigatorState>();

  // Use localhost for Chrome/Web; use 10.0.2.2 for Android Emulator
  static const String baseUrl = 'http://localhost:3000';

  // Timeout-enabled client
  final http.Client _client = http.Client();

  /// Build headers with JWT token if available.
  Future<Map<String, String>> _authHeaders() async {
    final prefs = await SharedPreferences.getInstance();
    final token = prefs.getString('jwt_token');
    final headers = <String, String>{
      'Content-Type': 'application/json',
    };
    if (token != null) {
      headers['Authorization'] = 'Bearer $token';
    }
    return headers;
  }

  /// Handle responses, throwing appropriate exceptions.
  T _handleResponse<T>(http.Response response, T Function(dynamic) parse) {
    if (response.statusCode == 401) {
      // 自动清除 token 并跳转登录页
      _clearAuthAndRedirect();
      throw AuthException('Session expired. Please login again.');
    }
    if (response.statusCode == 409) {
      String msg = 'Resource conflict.';
      try {
        final body = jsonDecode(response.body);
        msg = body['message']?.toString() ?? msg;
      } catch (_) {}
      throw ConflictException(msg);
    }
    if (response.statusCode == 403) {
      throw AuthException('Permission denied.');
    }
    if (response.statusCode >= 500) {
      throw ServerException(response.statusCode);
    }
    if (response.statusCode != 200) {
      String msg = 'Request failed: $response.statusCode';
      try {
        final body = jsonDecode(response.body);
        msg = body['message']?.toString() ?? msg;
      } catch (_) {}
      throw NetworkException(msg);
    }
    return parse(jsonDecode(response.body));
  }

  Future<void> _clearAuthAndRedirect() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove('jwt_token');
    await prefs.remove('refresh_token');
    if (navigatorKey.currentState != null) {
      navigatorKey.currentState!.pushReplacementNamed('/login');
    }
  }

  /// GET request with 15s timeout.
  Future<http.Response> _get(Uri url, Map<String, String> headers) async {
    return _client.get(url, headers: headers).timeout(
      const Duration(seconds: 15),
      onTimeout: () => throw NetworkException('请求超时，请稍后重试'),
    );
  }

  /// POST request with 30s timeout.
  Future<http.Response> _post(Uri url, Map<String, String> headers, String body) async {
    return _client.post(url, headers: headers, body: body).timeout(
      const Duration(seconds: 30),
      onTimeout: () => throw NetworkException('请求超时，请稍后重试'),
    );
  }

  Future<String> sendChatMessage(ChatMessage message) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/chat'),
      headers,
      jsonEncode(message.toJson()),
    );

    return _handleResponse(response, (data) => data['reply'] ?? 'Empty response');
  }

  Future<String> login(String username, String password) async {
    final response = await _post(
      Uri.parse('$baseUrl/api/auth/login'),
      {'Content-Type': 'application/json'},
      jsonEncode({'username': username, 'password': password}),
    );

    if (response.statusCode == 200) {
      final data = jsonDecode(response.body);
      final token = data['token'] ?? '';
      if (token.isEmpty) {
        throw Exception(data['message'] ?? 'Login failed');
      }
      return token;
    } else {
      throw Exception('Login error: ${response.statusCode}');
    }
  }

  Future<String> register(String username, String password) async {
    final response = await _post(
      Uri.parse('$baseUrl/api/auth/register'),
      {'Content-Type': 'application/json'},
      jsonEncode({'username': username, 'password': password}),
    );

    if (response.statusCode == 200) {
      final data = jsonDecode(response.body);
      final token = data['token'] ?? '';
      if (token.isEmpty) {
        throw Exception(data['message'] ?? 'Registration failed');
      }
      return token;
    } else {
      throw Exception('Registration error: ${response.statusCode}');
    }
  }

  Future<Map<String, dynamic>> getUserProfile() async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/user/profile'),
      headers,
    );

    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> getUserListings(
      {int limit = 20, int offset = 0}) async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/user/listings?limit=$limit&offset=$offset'),
      headers,
    );

    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  // ---------------------------------------------------------------------------
  // Marketplace browse / detail / create
  // ---------------------------------------------------------------------------

  Future<ListingsResponse> getListings({
    int limit = 20,
    int offset = 0,
    String? category,
    String? search,
  }) async {
    final headers = await _authHeaders();
    final queryParams = <String, String>{
      'limit': limit.toString(),
      'offset': offset.toString(),
    };
    if (category != null) queryParams['category'] = category;
    if (search != null && search.isNotEmpty) queryParams['search'] = search;

    final uri = Uri.parse('$baseUrl/api/listings').replace(
      queryParameters: queryParams,
    );
    final response = await _get(uri, headers);
    return _handleResponse(response, (data) => ListingsResponse.fromJson(data));
  }

  Future<Listing> getListingDetail(String id) async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/listings/$id'),
      headers,
    );
    return _handleResponse(
        response, (data) => Listing.fromJson(data as Map<String, dynamic>));
  }

  Future<String> createListing({
    required String title,
    required String category,
    required String brand,
    required int conditionScore,
    required double suggestedPriceCny,
    required List<String> defects,
    String? description,
  }) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/listings'),
      headers,
      jsonEncode({
        'title': title,
        'category': category,
        'brand': brand,
        'condition_score': conditionScore,
        'suggested_price_cny': suggestedPriceCny,
        'defects': defects,
        'description': description,
      }),
    );
    return _handleResponse(response, (data) => data['id'] ?? '');
  }

  Future<RecognizedItem> recognizeItem(String imageBase64) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/listings/recognize'),
      headers,
      jsonEncode({'image_base64': imageBase64}),
    );
    return _handleResponse(response, (data) => RecognizedItem.fromJson(data));
  }

  // ---------------------------------------------------------------------------
  // Orders
  // ---------------------------------------------------------------------------

  Future<Map<String, dynamic>> getOrders({String? role, int limit = 20, int offset = 0}) async {
    final headers = await _authHeaders();
    final queryParams = <String, String>{
      'limit': limit.toString(),
      'offset': offset.toString(),
    };
    if (role != null) queryParams['role'] = role;
    final uri = Uri.parse('$baseUrl/api/orders').replace(queryParameters: queryParams);
    final response = await _get(uri, headers);
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> getOrder(String orderId) async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/orders/$orderId'),
      headers,
    );
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> createOrder({
    required String listingId,
    required double offeredPriceCny,
  }) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/orders'),
      headers,
      jsonEncode({
        'listing_id': listingId,
        'offered_price_cny': offeredPriceCny,
      }),
    );
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<void> payOrder(String orderId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/orders/$orderId/pay'),
      headers,
      '{}',
    );
    _handleResponse(response, (_) {});
  }

  Future<void> shipOrder(String orderId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/orders/$orderId/ship'),
      headers,
      '{}',
    );
    _handleResponse(response, (_) {});
  }

  Future<void> confirmOrder(String orderId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/orders/$orderId/confirm'),
      headers,
      '{}',
    );
    _handleResponse(response, (_) {});
  }

  Future<void> cancelOrder(String orderId, {String? reason}) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/orders/$orderId/cancel'),
      headers,
      jsonEncode({'reason': reason}),
    );
    _handleResponse(response, (_) {});
  }

  // ---------------------------------------------------------------------------
  // Watchlist
  // ---------------------------------------------------------------------------

  Future<List<dynamic>> getWatchlist() async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/watchlist'),
      headers,
    );
    return _handleResponse(response, (data) => data as List<dynamic>);
  }

  Future<void> addToWatchlist(String listingId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/watchlist/$listingId'),
      headers,
      '{}',
    );
    _handleResponse(response, (_) {});
  }

  Future<void> removeFromWatchlist(String listingId) async {
    final headers = await _authHeaders();
    final response = await http.delete(
      Uri.parse('$baseUrl/api/watchlist/$listingId'),
      headers: headers,
    );
    _handleResponse(response, (_) {});
  }

  Future<bool> isWatched(String listingId) async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/watchlist/$listingId'),
      headers,
    );
    final data = _handleResponse(response, (d) => d as Map<String, dynamic>);
    return data['watched'] ?? false;
  }

  // ---------------------------------------------------------------------------
  // Conversations
  // ---------------------------------------------------------------------------

  Future<List<dynamic>> getConversations() async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/conversations'),
      headers,
    );
    return _handleResponse(response, (data) => data as List<dynamic>);
  }

  Future<Map<String, dynamic>> getConversationMessages(String conversationId, {int limit = 50, int offset = 0}) async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/conversations/$conversationId/messages?limit=$limit&offset=$offset'),
      headers,
    );
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  // ---------------------------------------------------------------------------
  // Users
  // ---------------------------------------------------------------------------

  Future<Map<String, dynamic>> getPublicUserProfile(String userId) async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/users/$userId'),
      headers,
    );
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> searchUsers(String query, {int limit = 20, int offset = 0}) async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/users/search?q=$query&limit=$limit&offset=$offset'),
      headers,
    );
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  // ---------------------------------------------------------------------------
  // Admin endpoints (role = 'admin' required)
  // ---------------------------------------------------------------------------

  Future<void> banUser(String userId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/admin/users/$userId/ban'),
      headers,
      '{}',
    );
    _handleResponse(response, (_) => null);
  }

  Future<void> unbanUser(String userId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/admin/users/$userId/unban'),
      headers,
      '{}',
    );
    _handleResponse(response, (_) => null);
  }

  Future<void> takedownListing(String listingId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/admin/listings/$listingId/takedown'),
      headers,
      '{}',
    );
    _handleResponse(response, (_) => null);
  }

  Future<Map<String, dynamic>> getAdminStats() async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/admin/stats'),
      headers,
    );
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> getAdminUsers({String? q, int limit = 20, int offset = 0}) async {
    final headers = await _authHeaders();
    final queryParams = <String, String>{
      'limit': limit.toString(),
      'offset': offset.toString(),
    };
    if (q != null && q.isNotEmpty) queryParams['q'] = q;
    final uri = Uri.parse('$baseUrl/api/admin/users').replace(queryParameters: queryParams);
    final response = await _get(uri, headers);
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> getAdminListings({String? status, int limit = 50, int offset = 0}) async {
    final headers = await _authHeaders();
    final queryParams = <String, String>{
      'limit': limit.toString(),
      'offset': offset.toString(),
    };
    if (status != null) queryParams['status'] = status;
    final uri = Uri.parse('$baseUrl/api/admin/listings').replace(queryParameters: queryParams);
    final response = await _get(uri, headers);
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> getAdminOrders({String? status, int limit = 50, int offset = 0}) async {
    final headers = await _authHeaders();
    final queryParams = <String, String>{
      'limit': limit.toString(),
      'offset': offset.toString(),
    };
    if (status != null) queryParams['status'] = status;
    final uri = Uri.parse('$baseUrl/api/admin/orders').replace(queryParameters: queryParams);
    final response = await _get(uri, headers);
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  // ---------------------------------------------------------------------------
  // Stats
  // ---------------------------------------------------------------------------

  Future<Map<String, dynamic>> getStats() async {
    final response = await _get(
      Uri.parse('$baseUrl/api/stats'),
      {'Content-Type': 'application/json'},
    );
    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  // ---------------------------------------------------------------------------
  // Listings management
  // ---------------------------------------------------------------------------

  Future<void> updateListing(String id, Map<String, dynamic> updates) async {
    final headers = await _authHeaders();
    final response = await http.put(
      Uri.parse('$baseUrl/api/listings/$id'),
      headers: headers,
      body: jsonEncode(updates),
    );
    _handleResponse(response, (_) {});
  }

  Future<void> deleteListing(String id) async {
    final headers = await _authHeaders();
    final response = await http.delete(
      Uri.parse('$baseUrl/api/listings/$id'),
      headers: headers,
    );
    _handleResponse(response, (_) {});
  }

  Future<void> changePassword(String currentPassword, String newPassword) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/auth/change-password'),
      headers,
      jsonEncode({
        'current_password': currentPassword,
        'new_password': newPassword,
      }),
    );
    _handleResponse(response, (_) {});
  }

  // ---------------------------------------------------------------------------
  // Negotiations (HITL)
  // ---------------------------------------------------------------------------

  /// List pending negotiation requests for the current user.
  /// Sellers see pending + expired; buyers see countered + approved + rejected + expired.
  Future<List<HitlRequest>> getNegotiations() async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/negotiations'),
      headers,
    );
    final data = _handleResponse(response, (d) => d as Map<String, dynamic>);
    final items = data['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => HitlRequest.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  /// Seller responds to a pending negotiation.
  /// action: 'approve' | 'reject' | 'counter'
  /// counter_price: required when action == 'counter' (in yuan, not cents)
  Future<Map<String, dynamic>> respondNegotiation(
    String id, {
    required String action,
    double? counterPrice,
  }) async {
    final headers = await _authHeaders();
    final body = <String, dynamic>{'action': action};
    if (counterPrice != null) body['counter_price'] = counterPrice;
    final response = await http.patch(
      Uri.parse('$baseUrl/api/negotiations/$id/respond'),
      headers: headers,
      body: jsonEncode(body),
    );
    return _handleResponse(response, (d) => d as Map<String, dynamic>);
  }

  /// Buyer accepts seller's counter-offer.
  Future<Map<String, dynamic>> acceptCounterNegotiation(String id) async {
    final headers = await _authHeaders();
    final response = await http.patch(
      Uri.parse('$baseUrl/api/negotiations/$id/accept'),
      headers: headers,
      body: '{}',
    );
    return _handleResponse(response, (d) => d as Map<String, dynamic>);
  }

  /// Buyer rejects seller's counter-offer.
  Future<Map<String, dynamic>> rejectCounterNegotiation(String id) async {
    final headers = await _authHeaders();
    final response = await http.patch(
      Uri.parse('$baseUrl/api/negotiations/$id/reject'),
      headers: headers,
      body: '{}',
    );
    return _handleResponse(response, (d) => d as Map<String, dynamic>);
  }

  // ---------------------------------------------------------------------------
  // Chat connections (three-way handshake)
  // ---------------------------------------------------------------------------

  /// 获取会话列表
  Future<List<Conversation>> getConnections() async {
    final headers = await _authHeaders();
    final response = await _get(
      Uri.parse('$baseUrl/api/chat/connections'),
      headers,
    );
    final data = _handleResponse(response, (d) => d as Map<String, dynamic>);
    final items = data['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => Conversation.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  /// 请求建立连接
  Future<void> requestConnection(String receiverId, {String? listingId}) async {
    final headers = await _authHeaders();
    final body = <String, dynamic>{'receiver_id': receiverId};
    if (listingId != null) body['listing_id'] = listingId;
    final response = await _post(
      Uri.parse('$baseUrl/api/chat/connect/request'),
      headers,
      jsonEncode(body),
    );
    _handleResponse(response, (_) {});
  }

  /// 接受连接
  Future<void> acceptConnection(String connectionId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/chat/connect/accept'),
      headers,
      jsonEncode({'connection_id': connectionId}),
    );
    _handleResponse(response, (_) {});
  }

  /// 拒绝连接
  Future<void> rejectConnection(String connectionId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/chat/connect/reject'),
      headers,
      jsonEncode({'connection_id': connectionId}),
    );
    _handleResponse(response, (_) {});
  }

  /// 发送私聊消息
  Future<ConversationMessage> sendMessage(
    String conversationId, {
    required String content,
    String? imageBase64,
    String? audioBase64,
  }) async {
    final headers = await _authHeaders();
    final body = <String, dynamic>{'content': content};
    if (imageBase64 != null) body['image_base64'] = imageBase64;
    if (audioBase64 != null) body['audio_base64'] = audioBase64;

    final response = await _post(
      Uri.parse('$baseUrl/api/chat/conversations/$conversationId/messages'),
      headers,
      jsonEncode(body),
    );
    final data = _handleResponse(
      response,
      (d) => ConversationMessage.fromJson(d as Map<String, dynamic>),
    );
    return data;
  }

  /// 获取私聊消息列表
  Future<List<ConversationMessage>> getChatConversationMessages(
    String conversationId, {
    int limit = 50,
    int offset = 0,
  }) async {
    final headers = await _authHeaders();
    final uri = Uri.parse(
      '$baseUrl/api/chat/conversations/$conversationId/messages?limit=$limit&offset=$offset',
    );
    final response = await _get(uri, headers);
    final data = _handleResponse(response, (d) => d as Map<String, dynamic>);
    final messages = data['messages'] as List<dynamic>? ?? [];
    return messages
        .map((e) => ConversationMessage.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  /// 标记消息已读
  Future<void> markMessageRead(String messageId) async {
    final headers = await _authHeaders();
    final response = await _post(
      Uri.parse('$baseUrl/api/chat/messages/$messageId/read'),
      headers,
      '{}',
    );
    _handleResponse(response, (_) {});
  }
}

class RecognizedItem {
  final String title;
  final String category;
  final String brand;
  final int conditionScore;
  final List<String> defects;
  final String description;

  RecognizedItem({
    required this.title,
    required this.category,
    required this.brand,
    required this.conditionScore,
    required this.defects,
    required this.description,
  });

  factory RecognizedItem.fromJson(Map<String, dynamic> json) {
    return RecognizedItem(
      title: json['title'] ?? '',
      category: json['category'] ?? 'other',
      brand: json['brand'] ?? '',
      conditionScore: json['condition_score'] ?? 5,
      defects: (json['defects'] as List<dynamic>?)?.map((e) => e.toString()).toList() ?? [],
      description: json['description'] ?? '',
    );
  }
}
