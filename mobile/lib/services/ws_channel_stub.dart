import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel connectWsChannel({
  required String wsUrl,
  required String token,
}) {
  throw UnsupportedError(
    'WebSocket auth requires platform-specific channel support.',
  );
}
