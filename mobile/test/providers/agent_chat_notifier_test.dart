import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:good4ncu_mobile/providers/agent_chat_notifier.dart';
import 'package:good4ncu_mobile/services/sse_service.dart';

class _FakeSseService extends SseService {
  final StreamController<SseToken> _controller = StreamController<SseToken>();

  String? lastMessage;
  String? lastConversationId;
  bool disconnected = false;

  @override
  Stream<SseToken> get stream => _controller.stream;

  @override
  Future<void> connect({
    required String message,
    String? conversationId,
    String? listingId,
    String? imageUrl,
    String? audioUrl,
  }) async {
    lastMessage = message;
    lastConversationId = conversationId;
  }

  @override
  Future<void> disconnect() async {
    disconnected = true;
  }

  @override
  void dispose() {
    _controller.close();
  }
}

void main() {
  group('AgentChatNotifier', () {
    test(
      'requestGreeting adds a local agent greeting without SSE connect',
      () async {
        final fakeSse = _FakeSseService();
        final notifier = AgentChatNotifier(sseService: fakeSse);

        await notifier.requestGreeting();

        expect(fakeSse.lastConversationId, isNull);
        expect(fakeSse.lastMessage, isNull);

        final state = notifier.state;
        expect(state, isA<AgentChatLoaded>());
        final loaded = state as AgentChatLoaded;
        expect(loaded.messages.length, 1);
        expect(loaded.messages.first.isFromAgent, isTrue);
        expect(loaded.messages.first.content.trim().isNotEmpty, isTrue);

        notifier.dispose();
      },
    );

    test('requestGreeting is idempotent when messages already exist', () async {
      final fakeSse = _FakeSseService();
      final notifier = AgentChatNotifier(sseService: fakeSse);

      await notifier.requestGreeting();
      await notifier.requestGreeting();

      final state = notifier.state;
      expect(state, isA<AgentChatLoaded>());
      final loaded = state as AgentChatLoaded;
      expect(loaded.messages.length, 1);
      expect(fakeSse.lastMessage, isNull);

      notifier.dispose();
    });
  });
}
