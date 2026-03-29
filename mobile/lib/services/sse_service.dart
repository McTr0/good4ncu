import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import '../utils/platform_utils.dart';
import 'token_storage.dart';

/// SSE token event parsed from `data: {...}\n\n` format.
class SseToken {
  final String token;
  final String conversationId;

  /// True when this is a complete message (e.g. greeting) rather than a streaming token.
  /// Complete messages should be finalized immediately without waiting for [DONE].
  final bool isComplete;
  final String? error;

  SseToken({
    required this.token,
    required this.conversationId,
    this.isComplete = false,
    this.error,
  });

  factory SseToken.fromJson(Map<String, dynamic> json) {
    return SseToken(
      token: json['token'] ?? '',
      conversationId: json['conversation_id'] ?? '',
      isComplete: json['is_complete'] as bool? ?? false,
    );
  }
}

/// Server-Sent Events stream consumer.
///
/// Connects to `GET /api/chat/stream` with JWT auth.
/// Each SSE `data:` line is parsed as JSON and emitted via StreamController.
class SseService {
  static String get _baseUrl => getApiBaseUrl();
  static const int _maxPendingChars = 65536;

  http.Client? _client;
  StreamController<SseToken>? _controller;
  StreamSubscription<String>? _streamSubscription;
  bool _isConnected = false;
  int _activeConnectionId = 0;

  /// Stream of parsed SSE token events.
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
    await disconnect();
    final connectionId = _activeConnectionId;

    final token = await TokenStorage.instance.getAccessToken();
    if (token == null) {
      throw Exception('SSE: No JWT token — not authenticated');
    }

    final client = http.Client();
    _client = client;
    // Single-subscription stream preserves events that arrive before UI listener attaches.
    _controller = StreamController<SseToken>();

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

    final streamedResponse = await client.send(request);

    if (connectionId != _activeConnectionId) {
      client.close();
      return;
    }

    if (streamedResponse.statusCode != 200) {
      await _closeController(connectionId: connectionId);
      throw Exception('SSE connection failed: ${streamedResponse.statusCode}');
    }

    _isConnected = true;
    var pendingSseText = '';

    _streamSubscription = streamedResponse.stream
        .transform(utf8.decoder)
        .listen(
          (decodedChunk) {
            if (connectionId != _activeConnectionId) {
              return;
            }
            pendingSseText = _appendDecodedText(
              connectionId: connectionId,
              decodedChunk: decodedChunk,
              pendingText: pendingSseText,
            );
          },
          onError: (error) {
            if (connectionId != _activeConnectionId) {
              return;
            }
            _isConnected = false;
            _streamSubscription = null;
            _emitError(connectionId, error);
            unawaited(_closeController(connectionId: connectionId));
          },
          onDone: () {
            if (connectionId != _activeConnectionId) {
              return;
            }
            _isConnected = false;
            _streamSubscription = null;
            unawaited(_closeController(connectionId: connectionId));
          },
          cancelOnError: false,
        );
  }

  void _emitToken(int connectionId, SseToken token) {
    if (connectionId != _activeConnectionId) {
      return;
    }
    final controller = _controller;
    if (controller == null || controller.isClosed) {
      return;
    }
    try {
      controller.add(token);
    } catch (_) {
      // Ignore stale emissions racing with disconnect.
    }
  }

  void _emitError(int connectionId, Object error) {
    if (connectionId != _activeConnectionId) {
      return;
    }
    final controller = _controller;
    if (controller == null || controller.isClosed) {
      return;
    }
    try {
      controller.addError(error);
    } catch (_) {
      // Ignore stale emissions racing with disconnect.
    }
  }

  Future<void> _closeController({int? connectionId}) async {
    if (connectionId != null && connectionId != _activeConnectionId) {
      return;
    }
    final controller = _controller;
    _controller = null;
    if (controller == null || controller.isClosed) {
      return;
    }
    await controller.close();
  }

  String _appendDecodedText({
    required int connectionId,
    required String decodedChunk,
    required String pendingText,
  }) {
    if (connectionId != _activeConnectionId) {
      return pendingText;
    }
    if (decodedChunk.isEmpty) {
      return pendingText;
    }

    final normalized = decodedChunk
        .replaceAll('\r\n', '\n')
        .replaceAll('\r', '\n');
    var updatedPendingText = pendingText + normalized;

    while (true) {
      final separatorIndex = updatedPendingText.indexOf('\n\n');
      if (separatorIndex < 0) {
        break;
      }

      final eventBlock = updatedPendingText.substring(0, separatorIndex);
      updatedPendingText = updatedPendingText.substring(separatorIndex + 2);
      _processSseEventBlock(connectionId, eventBlock);
    }

    // Keep memory bounded if server sends malformed chunks without separators.
    if (updatedPendingText.length > _maxPendingChars) {
      updatedPendingText = updatedPendingText.substring(
        updatedPendingText.length - _maxPendingChars,
      );
    }

    return updatedPendingText;
  }

  void _processSseEventBlock(int connectionId, String eventBlock) {
    if (eventBlock.trim().isEmpty) {
      return;
    }

    final lines = eventBlock.split('\n');
    final dataLines = <String>[];

    for (final line in lines) {
      final trimmed = line.trimRight();
      if (trimmed.startsWith('data:')) {
        dataLines.add(trimmed.substring(5).trimLeft());
      }
    }

    if (dataLines.isEmpty) {
      return;
    }

    final payload = dataLines.join('\n').trim();
    if (payload.isEmpty) {
      return;
    }

    final decoded = _decodeSseData(payload);
    if (decoded != null) {
      _emitToken(connectionId, decoded);
    }
  }

  /// Decode a single SSE data field using proper JSON parsing.
  /// Handles:
  /// - Streaming tokens: `{"token": "...", "conversation_id": "..."}`
  /// - Complete messages (greeting): `{"content": "...", "is_complete": true}`
  /// - Error events: `{"error": "..."}`
  SseToken? _decodeSseData(String jsonStr) {
    if (jsonStr.isEmpty) return null;
    try {
      final decoded = jsonDecode(jsonStr) as Map<String, dynamic>;

      // Handle error events from backend.
      final error = decoded['error'] as String?;
      if (error != null) {
        return SseToken(
          token: '',
          conversationId: decoded['conversation_id'] as String? ?? '',
          isComplete: true,
          error: error,
        );
      }

      // Complete message — greeting or direct response sent as a single event.
      // Backend sends `content` field (not `token`) for these.
      final content = decoded['content'] as String?;
      if (content != null && content.isNotEmpty) {
        return SseToken(
          token: content,
          conversationId: decoded['conversation_id'] as String? ?? '',
          isComplete: true, // Finalize immediately — no [DONE] will follow.
        );
      }

      // Streaming token.
      final token = decoded['token'] as String?;
      if (token != null && token.isNotEmpty) {
        return SseToken.fromJson(decoded);
      }

      return null;
    } catch (e) {
      debugPrint('SSE JSON parse error: $e — raw: $jsonStr');
      return null;
    }
  }

  /// Disconnect and clean up. Idempotent.
  Future<void> disconnect() async {
    _activeConnectionId += 1;
    _isConnected = false;
    await _streamSubscription?.cancel();
    _streamSubscription = null;
    _client?.close();
    _client = null;
    await _closeController();
  }

  void dispose() {
    disconnect();
  }
}
