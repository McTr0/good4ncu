import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import '../utils/platform_utils.dart';
import 'token_storage.dart';

/// SSE token event parsed from `data: {...}\n\n` format.
class SseToken {
  final String token;
  final String conversationId;
  final String? hitlRequestId;
  final String? eventType;

  SseToken({
    required this.token,
    required this.conversationId,
    this.hitlRequestId,
    this.eventType,
  });

  factory SseToken.fromJson(Map<String, dynamic> json) {
    return SseToken(
      token: json['token'] ?? '',
      conversationId: json['conversation_id'] ?? '',
      hitlRequestId: json['hitl_request_id'],
      eventType: json['event_type'],
    );
  }
}

/// Server-Sent Events stream consumer.
///
/// Connects to `GET /api/chat/stream` with JWT auth.
/// Each SSE `data:` line is parsed as JSON and emitted via `StreamController`.
class SseService {
  static String get _baseUrl => getApiBaseUrl();

  http.Client? _client;
  StreamController<SseToken>? _controller;
  bool _isConnected = false;

  /// Stream of parsed SSE token events. Emits one token per LLM word/phrase.
  Stream<SseToken> get stream => _controller?.stream ?? const Stream.empty();

  bool get isConnected => _isConnected;

  /// Connect to SSE stream. Idempotent — safe to call multiple times.
  Future<void> connect({
    required String message,
    String? conversationId,
    String? listingId,
    String? imageBase64,
    String? audioBase64,
  }) async {
    // Disconnect any existing connection first.
    await disconnect();

    final token = await TokenStorage.instance.getAccessToken();
    if (token == null) {
      throw Exception('SSE: No JWT token — not authenticated');
    }

    _client = http.Client();
    _controller = StreamController<SseToken>.broadcast();

    // Build query params for GET /api/chat/stream.
    // JWT is sent via Authorization header.
    final queryParams = <String, String>{'message': message};
    if (conversationId != null) queryParams['conversation_id'] = conversationId;
    if (listingId != null) queryParams['listing_id'] = listingId;
    if (imageBase64 != null) queryParams['image'] = imageBase64;
    if (audioBase64 != null) queryParams['audio'] = audioBase64;

    final uri = Uri.parse(
      '$_baseUrl/api/chat/stream',
    ).replace(queryParameters: queryParams);

    final request = http.Request('GET', uri);
    request.headers['Accept'] = 'text/event-stream';
    request.headers['Cache-Control'] = 'no-cache';
    request.headers['Authorization'] = 'Bearer $token';

    // Start the request — this is a streaming response, not a regular HTTP call.
    // `send()` returns a Future<StreamedResponse> once headers are received.
    final streamedResponse = await _client!.send(request);

    if (streamedResponse.statusCode != 200) {
      await _controller?.close();
      throw Exception('SSE connection failed: ${streamedResponse.statusCode}');
    }

    _isConnected = true;

    // Read the streamed body as bytes and parse SSE data lines.
    // SSE format: "data: {...}\n\n" — we accumulate a buffer and emit
    // on each `\n\n` (double newline = end of event).
    final byteStream = streamedResponse.stream;
    final buffer = <int>[];

    byteStream.listen(
      (chunk) {
        buffer.addAll(chunk);
        _processBuffer(buffer);
      },
      onError: (error) {
        _isConnected = false;
        _controller?.addError(error);
      },
      onDone: () {
        _isConnected = false;
        _controller?.close();
      },
      cancelOnError: false,
    );
  }

  /// Process the buffer, extracting complete SSE events.
  void _processBuffer(List<int> buffer) {
    final raw = String.fromCharCodes(buffer);
    final lines = raw.split('\n');
    int consumeCount = 0;

    for (final line in lines) {
      consumeCount += line.length + 1; // +1 for newline
      if (line.startsWith('data:')) {
        final jsonStr = line.substring(5).trim();
        if (jsonStr.isEmpty) continue;
        try {
          // SSE events are separated by blank lines (double newline).
          // We emit on each `data:` line; the caller can handle grouping.
          final decoded = _decodeSseData(jsonStr);
          if (decoded != null) {
            _controller?.add(decoded);
          }
        } catch (e) {
          debugPrint('SSE parse error: $e — raw: $jsonStr');
        }
      }
    }

    // Keep unprocessed bytes in buffer (shouldn't happen with line split, but safety first)
    if (consumeCount < buffer.length) {
      buffer.removeRange(0, consumeCount);
    } else {
      buffer.clear();
    }
  }

  /// Decode a single SSE data field. Handles JSON with optional `event_type` field.
  SseToken? _decodeSseData(String jsonStr) {
    // SSE data can contain JSON object or a plain string.
    if (jsonStr.startsWith('{')) {
      // Parse JSON fields directly without full parser.
      try {
        return SseToken(
          token: _extractJsonField(jsonStr, 'token') ?? '',
          conversationId: _extractJsonField(jsonStr, 'conversation_id') ?? '',
          hitlRequestId: _extractJsonField(jsonStr, 'hitl_request_id'),
          eventType: _extractJsonField(jsonStr, 'event_type'),
        );
      } catch (_) {
        return null;
      }
    }
    // Plain string data — wrap as token
    return SseToken(token: jsonStr, conversationId: '');
  }

  /// Extract a field from a compact JSON string without full parser.
  String? _extractJsonField(String json, String field) {
    final pattern = '"$field":"';
    final idx = json.indexOf(pattern);
    if (idx == -1) {
      // Try without quotes (numeric or boolean)
      final pat2 = '"$field":';
      final idx2 = json.indexOf(pat2);
      if (idx2 == -1) return null;
      final start = idx2 + pat2.length;
      if (start >= json.length) return null;
      if (json[start] == '"') {
        // String value — find closing quote
        final end = json.indexOf('"', start + 1);
        if (end == -1) return null;
        return json.substring(start + 1, end);
      }
      return null;
    }
    final start = idx + pattern.length;
    final end = json.indexOf('"', start);
    if (end == -1) return null;
    return json.substring(start, end);
  }

  /// Disconnect and clean up. Idempotent.
  Future<void> disconnect() async {
    _isConnected = false;
    _client?.close();
    _client = null;
    await _controller?.close();
    _controller = null;
  }

  void dispose() {
    disconnect();
  }
}
