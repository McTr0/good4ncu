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

class ApiService {
  // Global navigator key for programmatic navigation (e.g., force logout)
  static final GlobalKey<NavigatorState> navigatorKey = GlobalKey<NavigatorState>();

  // Use localhost for Chrome/Web; use 10.0.2.2 for Android Emulator
  static const String baseUrl = 'http://localhost:3000';

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

  /// Handle responses, throwing AuthException on 401.
  T _handleResponse<T>(http.Response response, T Function(dynamic) parse) {
    if (response.statusCode == 401) {
      throw AuthException('Session expired. Please login again.');
    }
    if (response.statusCode != 200) {
      throw Exception('Request failed: ${response.statusCode}');
    }
    return parse(jsonDecode(response.body));
  }

  Future<String> sendChatMessage(ChatMessage message) async {
    final headers = await _authHeaders();
    final response = await http.post(
      Uri.parse('$baseUrl/api/chat'),
      headers: headers,
      body: jsonEncode(message.toJson()),
    );

    return _handleResponse(response, (data) => data['reply'] ?? 'Empty response');
  }

  Future<String> login(String username, String password) async {
    final response = await http.post(
      Uri.parse('$baseUrl/api/auth/login'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode({'username': username, 'password': password}),
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
    final response = await http.post(
      Uri.parse('$baseUrl/api/auth/register'),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode({'username': username, 'password': password}),
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
    final response = await http.get(
      Uri.parse('$baseUrl/api/user/profile'),
      headers: headers,
    );

    return _handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  Future<Map<String, dynamic>> getUserListings(
      {int limit = 20, int offset = 0}) async {
    final headers = await _authHeaders();
    final response = await http.get(
      Uri.parse('$baseUrl/api/user/listings?limit=$limit&offset=$offset'),
      headers: headers,
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
    final response = await http.get(uri, headers: headers);
    return _handleResponse(response, (data) => ListingsResponse.fromJson(data));
  }

  Future<Listing> getListingDetail(String id) async {
    final headers = await _authHeaders();
    final response = await http.get(
      Uri.parse('$baseUrl/api/listings/$id'),
      headers: headers,
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
    final response = await http.post(
      Uri.parse('$baseUrl/api/listings'),
      headers: headers,
      body: jsonEncode({
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
    final response = await http.post(
      Uri.parse('$baseUrl/api/listings/recognize'),
      headers: headers,
      body: jsonEncode({'image_base64': imageBase64}),
    );
    return _handleResponse(response, (data) => RecognizedItem.fromJson(data));
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
