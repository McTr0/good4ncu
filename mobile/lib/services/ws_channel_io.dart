import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel connectWsChannel({
  required String wsUrl,
  required String token,
}) {
  return IOWebSocketChannel.connect(
    Uri.parse(wsUrl),
    headers: <String, dynamic>{'Authorization': 'Bearer $token'},
  );
}
