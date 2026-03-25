import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Incoming WS message envelope. Matches backend WS push payload.
class WsNotification {
  final String? id;
  final String eventType;
  final String title;
  final String body;
  final String? relatedOrderId;
  final String? relatedListingId;
  final String? negotiationId;
  /// 用于 connection_request 等事件
  final String? connectionId;
  /// 用于 new_message 和 message_read 事件
  final String? messageId;

  WsNotification({
    this.id,
    required this.eventType,
    required this.title,
    required this.body,
    this.relatedOrderId,
    this.relatedListingId,
    this.negotiationId,
    this.connectionId,
    this.messageId,
  });

  factory WsNotification.fromJson(Map<String, dynamic> json) {
    return WsNotification(
      id: json['id']?.toString(),
      eventType: json['event_type'] ?? '',
      title: json['title'] ?? '',
      body: json['body'] ?? '',
      relatedOrderId: json['related_order_id']?.toString(),
      relatedListingId: json['related_listing_id']?.toString(),
      negotiationId: json['negotiation_id']?.toString(),
      connectionId: json['connection_id']?.toString(),
      messageId: json['message_id']?.toString(),
    );
  }
}

/// WebSocket service with automatic reconnection (exponential backoff).
///
/// Connect once at app startup. `Stream<WsNotification>` delivers all push events.
class WsService {
  static const String _wsUrl = 'ws://localhost:3000/api/ws';

  /// Initial reconnect base delay (1 second).
  static const Duration _baseDelay = Duration(seconds: 1);
  /// Maximum reconnect delay cap (30 seconds).
  static const Duration _maxDelay = Duration(seconds: 30);

  WebSocketChannel? _channel;
  StreamController<WsNotification>? _controller;
  Timer? _heartbeatTimer;
  Timer? _reconnectTimer;

  int _reconnectAttempts = 0;
  bool _shouldReconnect = true;
  String? _pendingAuthToken;

  /// Stream of parsed WS push notifications.
  Stream<WsNotification> get stream => _controller?.stream ?? const Stream.empty();

  bool get isConnected => _channel != null;

  /// Connect to WebSocket. Stores token for reconnect use.
  Future<void> connect() async {
    final prefs = await SharedPreferences.getInstance();
    final token = prefs.getString('jwt_token');
    if (token == null) {
      throw Exception('WS: No JWT token — not authenticated');
    }
    _pendingAuthToken = token;
    _shouldReconnect = true;
    await _doConnect(token);
  }

  Future<void> _doConnect(String token) async {
    try {
      _channel = WebSocketChannel.connect(
        Uri.parse('$_wsUrl?token=$token'),
      );

      _controller ??= StreamController<WsNotification>.broadcast();

      // Start heartbeat monitor
      _startHeartbeatMonitor();

      _channel!.stream.listen(
        (data) => _handleMessage(data),
        onError: (error) {
          debugPrint('WS error: $error');
          _scheduleReconnect();
        },
        onDone: () {
          debugPrint('WS connection closed');
          _scheduleReconnect();
        },
        cancelOnError: false,
      );

      _reconnectAttempts = 0;
    } catch (e) {
      debugPrint('WS connect error: $e');
      _scheduleReconnect();
    }
  }

  void _handleMessage(dynamic data) {
    try {
      final json = jsonDecode(data as String) as Map<String, dynamic>;
      final eventType = json['event_type'] as String? ?? '';

      // Server sends ping as `{"type":"ping"}` — respond with pong via channel.
      if (eventType == 'ping' || (json['type']?.toString() == 'ping')) {
        _channel?.sink.add(jsonEncode({'type': 'pong'}));
        return;
      }

      // Parse notification payload.
      final notification = WsNotification.fromJson(json);
      _controller?.add(notification);
    } catch (e) {
      debugPrint('WS message parse error: $e — data: $data');
    }
  }

  void _startHeartbeatMonitor() {
    _heartbeatTimer?.cancel();
    // Heartbeat is handled by server-side ping/pong.
    // The stream's onError/onDone callbacks trigger reconnect on any issue.
  }

  void _scheduleReconnect() {
    if (!_shouldReconnect) return;
    if (_pendingAuthToken == null) return;

    _heartbeatTimer?.cancel();
    _channel = null;

    // Exponential backoff: 1s, 2s, 4s, 8s, ... cap at 30s.
    final delay = Duration(
      seconds: (_baseDelay.inSeconds * (1 << _reconnectAttempts))
          .clamp(1, _maxDelay.inSeconds),
    );
    _reconnectAttempts++;

    debugPrint('WS: scheduling reconnect in ${delay.inSeconds}s (attempt $_reconnectAttempts)');
    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(delay, () {
      if (_shouldReconnect && _pendingAuthToken != null) {
        _doConnect(_pendingAuthToken!);
      }
    });
  }

  /// Disconnect permanently. Call on logout.
  Future<void> disconnect() async {
    _shouldReconnect = false;
    _heartbeatTimer?.cancel();
    _reconnectTimer?.cancel();
    await _channel?.sink.close();
    _channel = null;
    // Don't close controller — stream consumers may still hold a reference.
  }

  void dispose() {
    disconnect();
  }
}
