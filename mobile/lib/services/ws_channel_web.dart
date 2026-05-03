import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel connectWsChannel({
  required String wsUrl,
  required String token,
}) {
  throw UnsupportedError(
    'Browser WebSocket connections are disabled until a secure auth ticket flow exists.',
  );
}
