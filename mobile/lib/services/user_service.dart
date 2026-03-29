import 'dart:convert';
import 'base_service.dart';

/// STS token response from GET /api/upload/token
class StsToken {
  final String accessKeyId;
  final String accessKeySecret;
  final String securityToken;
  final String expiration;
  final String endpoint;
  final String bucket;

  StsToken({
    required this.accessKeyId,
    required this.accessKeySecret,
    required this.securityToken,
    required this.expiration,
    required this.endpoint,
    required this.bucket,
  });

  factory StsToken.fromJson(Map<String, dynamic> json) {
    return StsToken(
      accessKeyId: json['access_key_id'] as String,
      accessKeySecret: json['access_key_secret'] as String,
      securityToken: json['security_token'] as String,
      expiration: json['expiration'] as String,
      endpoint: json['endpoint'] as String,
      bucket: json['bucket'] as String,
    );
  }
}

/// User service — handles profile, listings, public profile, and search.
class UserService extends BaseService {
  /// Get current user's profile.
  /// GET /api/user/profile
  Future<Map<String, dynamic>> getUserProfile() async {
    final headers = await authHeaders();
    final response = await get(
      Uri.parse('$baseUrl/api/user/profile'),
      headers,
    );
    return handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  /// Update current user's profile.
  /// PATCH /api/user/profile
  Future<Map<String, dynamic>> updateProfile({String? username, String? email, String? avatarUrl}) async {
    final headers = await authHeaders();
    final body = <String, dynamic>{};
    if (username != null) body['username'] = username;
    if (email != null) body['email'] = email;
    if (avatarUrl != null) body['avatar_url'] = avatarUrl;
    final response = await patch(
      Uri.parse('$baseUrl/api/user/profile'),
      headers,
      jsonEncode(body),
    );
    return handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  /// Get STS upload token for direct OSS upload.
  /// GET /api/upload/token
  Future<StsToken> getUploadToken() async {
    final headers = await authHeaders();
    final response = await get(
      Uri.parse('$baseUrl/api/upload/token'),
      headers,
    );
    return handleResponse(response, (data) => StsToken.fromJson(data as Map<String, dynamic>));
  }

  /// Get current user's listings.
  /// GET /api/user/listings
  Future<Map<String, dynamic>> getUserListings(
      {int limit = 20, int offset = 0}) async {
    final headers = await authHeaders();
    final response = await get(
      Uri.parse('$baseUrl/api/user/listings?limit=$limit&offset=$offset'),
      headers,
    );
    return handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  /// Get public user profile.
  /// GET /api/users/{id}
  Future<Map<String, dynamic>> getPublicUserProfile(String userId) async {
    final headers = await authHeaders();
    final response = await get(
      Uri.parse('$baseUrl/api/users/$userId'),
      headers,
    );
    return handleResponse(response, (data) => data as Map<String, dynamic>);
  }

  /// Search users by username.
  /// GET /api/users/search
  Future<Map<String, dynamic>> searchUsers(
    String query, {
    int limit = 20,
    int offset = 0,
  }) async {
    final headers = await authHeaders();
    final response = await get(
      Uri.parse('$baseUrl/api/users/search?q=$query&limit=$limit&offset=$offset'),
      headers,
    );
    return handleResponse(response, (data) => data as Map<String, dynamic>);
  }
}
