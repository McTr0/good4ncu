import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel connectWsChannel({
  required String wsUrl,
  required String token,
}) {
  return WebSocketChannel.connect(Uri.parse('$wsUrl?token=$token'));
}
