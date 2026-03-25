import 'dart:convert';
import 'package:http/http.dart' as http;
import 'package:shared_preferences/shared_preferences.dart';
import '../models/models.dart';

/// Service for fetching personalized recommendations via embedding similarity.
class RecommendationService {
  static const String _baseUrl = 'http://localhost:3000';

  Future<Map<String, String>> _authHeaders() async {
    final prefs = await SharedPreferences.getInstance();
    final token = prefs.getString('jwt_token');
    final headers = <String, String>{'Content-Type': 'application/json'};
    if (token != null) {
      headers['Authorization'] = 'Bearer $token';
    }
    return headers;
  }

  /// GET /api/recommendations/similar?listing_id=xxx
  /// Returns Top-10 similar listings based on pgvector cosine similarity.
  Future<List<Listing>> getSimilarListings(String listingId) async {
    final headers = await _authHeaders();
    final response = await http.get(
      Uri.parse('$_baseUrl/api/recommendations/similar?listing_id=$listingId'),
      headers: headers,
    );

    if (response.statusCode == 401) {
      throw Exception('请先登录');
    }
    if (response.statusCode != 200) {
      throw Exception('获取推荐失败: ${response.statusCode}');
    }

    final data = jsonDecode(response.body);
    final items = data['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => Listing.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  /// GET /api/recommendations/feed
  /// Returns personalized recommendation feed for the home page.
  Future<List<Listing>> getRecommendationFeed({int limit = 10}) async {
    final headers = await _authHeaders();
    final response = await http.get(
      Uri.parse('$_baseUrl/api/recommendations/feed?limit=$limit'),
      headers: headers,
    );

    if (response.statusCode == 401) {
      // Not logged in — return empty list, caller should not show carousel
      return [];
    }
    if (response.statusCode != 200) {
      throw Exception('获取推荐失败: ${response.statusCode}');
    }

    final data = jsonDecode(response.body);
    final items = data['items'] as List<dynamic>? ?? [];
    return items
        .map((e) => Listing.fromJson(e as Map<String, dynamic>))
        .toList();
  }
}
