import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:http/http.dart' as http;
import '../utils/platform_utils.dart';
import 'token_storage.dart';
import 'ws_service.dart';

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

/// Base class for all services — holds shared HTTP client and auth logic.
///
/// Subclasses can call the protected HTTP methods (get, post, put, patch, delete,
/// authHeaders, handleResponse, clearAuthAndRedirect).
class BaseService {
  static final GlobalKey<NavigatorState> navigatorKey =
      GlobalKey<NavigatorState>();
  static final http.Client _sharedClient = http.Client();

  String get baseUrl => getApiBaseUrl();

  http.Client get _client => _sharedClient;
  final TokenStorage _tokenStorage = TokenStorage.instance;

  @protected
  Future<Map<String, String>> authHeaders() async {
    final token = await _tokenStorage.getAccessToken();
    final headers = <String, String>{'Content-Type': 'application/json'};
    if (token != null) {
      headers['Authorization'] = 'Bearer $token';
    }
    return headers;
  }

  @protected
  T handleResponse<T>(http.Response response, T Function(dynamic) parse) {
    if (response.statusCode == 200) {
      try {
        return parse(jsonDecode(response.body));
      } catch (e) {
        throw ServerException(response.statusCode, '服务器返回数据格式错误');
      }
    }

    // --- Non-200: extract backend error message ---
    String serverMsg = '';
    try {
      final body = jsonDecode(response.body);
      serverMsg = (body['error'] ?? body['message'])?.toString() ?? '';
    } catch (_) {}

    if (response.statusCode == 401) {
      _clearAuthAndRedirect();
      throw AuthException(serverMsg.isNotEmpty ? serverMsg : '登录已过期，请重新登录');
    }
    if (response.statusCode == 403) {
      throw AuthException(serverMsg.isNotEmpty ? serverMsg : '您没有权限执行此操作');
    }
    if (response.statusCode == 409) {
      throw ConflictException(serverMsg.isNotEmpty ? serverMsg : '资源冲突');
    }
    if (response.statusCode >= 500) {
      throw ServerException(
        response.statusCode,
        serverMsg.isNotEmpty ? serverMsg : '服务器异常，请稍后再试',
      );
    }
    throw NetworkException(
      serverMsg.isNotEmpty ? serverMsg : '请求失败 (${response.statusCode})',
    );
  }

  Future<void> _clearAuthAndRedirect() async {
    await _tokenStorage.clearTokens();
    await WsService.instance.disconnect();
    final context = navigatorKey.currentContext;
    if (context != null && context.mounted) {
      GoRouter.of(context).go('/login');
    }
  }

  @protected
  Future<http.Response> get(Uri url, Map<String, String> headers) async {
    return _client
        .get(url, headers: headers)
        .timeout(
          const Duration(seconds: 15),
          onTimeout: () => throw NetworkException('请求超时，请稍后重试'),
        );
  }

  @protected
  Future<http.Response> post(
    Uri url,
    Map<String, String> headers,
    String body,
  ) async {
    return _client
        .post(url, headers: headers, body: body)
        .timeout(
          const Duration(seconds: 30),
          onTimeout: () => throw NetworkException('请求超时，请稍后重试'),
        );
  }

  @protected
  Future<http.Response> put(
    Uri url,
    Map<String, String> headers,
    String body,
  ) async {
    return _client
        .put(url, headers: headers, body: body)
        .timeout(
          const Duration(seconds: 15),
          onTimeout: () => throw NetworkException('请求超时，请稍后重试'),
        );
  }

  @protected
  Future<http.Response> patch(
    Uri url,
    Map<String, String> headers,
    String body,
  ) async {
    return _client
        .patch(url, headers: headers, body: body)
        .timeout(
          const Duration(seconds: 15),
          onTimeout: () => throw NetworkException('请求超时，请稍后重试'),
        );
  }

  @protected
  Future<http.Response> delete(Uri url, Map<String, String> headers) async {
    return _client
        .delete(url, headers: headers)
        .timeout(
          const Duration(seconds: 15),
          onTimeout: () => throw NetworkException('请求超时，请稍后重试'),
        );
  }

  // Token storage helpers (used by AuthService and ApiService)
  Future<String?> getToken() async {
    return _tokenStorage.getAccessToken();
  }

  Future<void> setToken(String token) async {
    await _tokenStorage.setAccessToken(token);
  }

  Future<String?> getRefreshToken() async {
    return _tokenStorage.getRefreshToken();
  }

  Future<void> setRefreshToken(String token) async {
    await _tokenStorage.setRefreshToken(token);
  }

  Future<void> clearRefreshToken() async {
    await _tokenStorage.removeRefreshToken();
  }

  Future<void> clearToken() async {
    await _tokenStorage.clearTokens();
  }
}
