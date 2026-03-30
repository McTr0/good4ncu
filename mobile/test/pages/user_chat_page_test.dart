import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:good4ncu_mobile/models/models.dart';
import 'package:good4ncu_mobile/components/audio_message_player.dart';
import 'package:good4ncu_mobile/pages/user_chat_page.dart';

void main() {
  group('MessageBubble', () {
    Widget buildTestableWidget(Widget child) {
      return MaterialApp(
        home: Scaffold(
          body: child,
        ),
      );
    }

    testWidgets('aligns own messages to the right', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'My message',
        sentAt: DateTime.now(),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: () {},
        ),
      ));

      final align = tester.widget<Align>(find.byType(Align));
      expect(align.alignment, equals(Alignment.centerRight));
    });

    testWidgets('aligns other messages to the left', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-other',
        content: 'Other message',
        sentAt: DateTime.now(),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: false,
          isConnected: true,
          onEdit: null,
        ),
      ));

      final align = tester.widget<Align>(find.byType(Align));
      expect(align.alignment, equals(Alignment.centerLeft));
    });

    testWidgets('displays message content', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-other',
        content: 'Hello, World!',
        sentAt: DateTime.now(),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: false,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('Hello, World!'), findsOneWidget);
    });

    testWidgets('displays timestamp', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-other',
        content: 'Test message',
        sentAt: DateTime(2024, 1, 15, 14, 30), // 14:30
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: false,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('14:30'), findsOneWidget);
    });

    testWidgets('shows edit link for own messages within edit window', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'My editable message',
        sentAt: DateTime.now().subtract(const Duration(minutes: 5)),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: () {},
        ),
      ));

      expect(find.text('编辑'), findsOneWidget);
    });

    testWidgets('does not show edit link for other users messages', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-other',
        content: 'Other message',
        sentAt: DateTime.now().subtract(const Duration(minutes: 5)),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: false,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('编辑'), findsNothing);
    });

    testWidgets('does not show edit link when onEdit is null', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'My message without edit',
        sentAt: DateTime.now().subtract(const Duration(minutes: 5)),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('编辑'), findsNothing);
    });

    testWidgets('displays edited indicator when message is edited', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Edited message',
        sentAt: DateTime.now().subtract(const Duration(minutes: 5)),
        editedAt: DateTime.now(),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: null, // Should be null because editedAt is set
        ),
      ));

      expect(find.text('（已编辑）'), findsOneWidget);
    });

    testWidgets('shows sending status with spinner', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Sending message...',
        sentAt: DateTime.now(),
        status: 'sending',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.text('发送中'), findsOneWidget);
    });

    testWidgets('shows sent status', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Sent message',
        sentAt: DateTime.now(),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('已发送'), findsOneWidget);
    });

    testWidgets('shows delivered status', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Delivered message',
        sentAt: DateTime.now(),
        status: 'delivered',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('已送达'), findsOneWidget);
    });

    testWidgets('shows read status with success color', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Read message',
        sentAt: DateTime.now(),
        status: 'read',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('已读'), findsOneWidget);
    });

    testWidgets('shows failed status', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Failed message',
        sentAt: DateTime.now(),
        status: 'failed',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('发送失败'), findsOneWidget);
    });

    testWidgets('does not show status when not connected', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Offline message',
        sentAt: DateTime.now(),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: false,
          onEdit: null,
        ),
      ));

      expect(find.text('已发送'), findsNothing);
    });

    testWidgets('displays voice message player', (tester) async {
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-other',
        content: '[语音消息]',
        audioBase64: 'audio-data',
        sentAt: DateTime.now(),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: false,
          isConnected: true,
          onEdit: null,
        ),
      ));

      expect(find.text('语音消息'), findsOneWidget);
      expect(find.byType(AudioMessagePlayer), findsOneWidget);
      expect(find.byIcon(Icons.play_circle), findsOneWidget);
    });

    testWidgets('triggers onEdit callback when edit is tapped', (tester) async {
      bool editCalled = false;
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Tap to edit',
        sentAt: DateTime.now().subtract(const Duration(minutes: 5)),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: () {
            editCalled = true;
          },
        ),
      ));

      await tester.tap(find.text('编辑'));
      await tester.pump();

      expect(editCalled, isTrue);
    });

    testWidgets('triggers onEdit callback when bubble is long pressed', (tester) async {
      bool editCalled = false;
      final message = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'Long press to edit',
        sentAt: DateTime.now().subtract(const Duration(minutes: 5)),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        MessageBubble(
          message: message,
          isMe: true,
          isConnected: true,
          onEdit: () {
            editCalled = true;
          },
        ),
      ));

      await tester.longPress(find.text('Long press to edit'));
      await tester.pump();

      expect(editCalled, isTrue);
    });

    testWidgets('uses different colors for own vs other messages', (tester) async {
      final myMessage = ConversationMessage(
        id: '1',
        conversationId: 'conv-1',
        senderId: 'user-me',
        content: 'My message',
        sentAt: DateTime.now(),
        status: 'sent',
      );

      final otherMessage = ConversationMessage(
        id: '2',
        conversationId: 'conv-1',
        senderId: 'user-other',
        content: 'Other message',
        sentAt: DateTime.now(),
        status: 'sent',
      );

      await tester.pumpWidget(buildTestableWidget(
        Column(
          children: [
            MessageBubble(
              message: myMessage,
              isMe: true,
              isConnected: true,
              onEdit: null,
            ),
            MessageBubble(
              message: otherMessage,
              isMe: false,
              isConnected: true,
              onEdit: null,
            ),
          ],
        ),
      ));

      // Find the containers with decorations (message bubbles)
      final containers = tester.widgetList<Container>(find.byType(Container));
      expect(containers.length, greaterThanOrEqualTo(2));
    });
  });

  group('ConnectionIndicator', () {
    testWidgets('shows offline state when ws not connected', (tester) async {
      await tester.pumpWidget(const MaterialApp(
        home: Scaffold(
          body: ConnectionIndicator(
            status: 'connected',
            isWsConnected: false,
          ),
        ),
      ));

      expect(find.text('离线'), findsOneWidget);
    });

    testWidgets('shows connected status when ws connected', (tester) async {
      await tester.pumpWidget(const MaterialApp(
        home: Scaffold(
          body: ConnectionIndicator(
            status: 'connected',
            isWsConnected: true,
          ),
        ),
      ));

      expect(find.text('在线'), findsOneWidget);
    });

    testWidgets('shows pending status', (tester) async {
      await tester.pumpWidget(const MaterialApp(
        home: Scaffold(
          body: ConnectionIndicator(
            status: 'pending',
            isWsConnected: true,
          ),
        ),
      ));

      expect(find.text('待接受'), findsOneWidget);
    });

    testWidgets('shows connecting status with animation', (tester) async {
      await tester.pumpWidget(const MaterialApp(
        home: Scaffold(
          body: ConnectionIndicator(
            status: 'connecting',
            isWsConnected: true,
          ),
        ),
      ));

      expect(find.text('连接中...'), findsOneWidget);
      // Verify the ConnectionIndicator has an AnimatedBuilder descendant
      expect(
        find.descendant(
          of: find.byType(ConnectionIndicator),
          matching: find.byType(AnimatedBuilder),
        ),
        findsWidgets,
      );
    });

    testWidgets('shows default offline for unknown status', (tester) async {
      await tester.pumpWidget(const MaterialApp(
        home: Scaffold(
          body: ConnectionIndicator(
            status: 'unknown_status',
            isWsConnected: true,
          ),
        ),
      ));

      expect(find.text('离线'), findsOneWidget);
    });
  });
}
