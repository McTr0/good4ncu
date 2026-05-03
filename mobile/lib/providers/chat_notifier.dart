import 'dart:async';
import 'package:state_notifier/state_notifier.dart';
import '../models/models.dart';
import '../services/chat_service.dart';
import '../services/user_service.dart';

/// Sealed state for chat messages — mutually exclusive states prevent boolean flag soup.
sealed class ChatViewState {
  const ChatViewState();
}

class ChatViewInitial extends ChatViewState {
  const ChatViewInitial();
}

class ChatViewLoading extends ChatViewState {
  const ChatViewLoading();
}

class ChatViewData extends ChatViewState {
  final List<ConversationMessage> messages;
  final String? currentUserId;
  final String? connectionStatus; // null, 'connecting', 'connected'
  final bool isOtherTyping;
  final String? editingMessageId;
  final bool isSending;
  const ChatViewData({
    required this.messages,
    this.currentUserId,
    this.connectionStatus,
    this.isOtherTyping = false,
    this.editingMessageId,
    this.isSending = false,
  });

  ChatViewData copyWith({
    List<ConversationMessage>? messages,
    String? currentUserId,
    String? connectionStatus,
    bool? isOtherTyping,
    String? editingMessageId,
    bool? isSending,
    bool clearEditing = false,
  }) {
    return ChatViewData(
      messages: messages ?? this.messages,
      currentUserId: currentUserId ?? this.currentUserId,
      connectionStatus: connectionStatus ?? this.connectionStatus,
      isOtherTyping: isOtherTyping ?? this.isOtherTyping,
      editingMessageId: clearEditing
          ? null
          : (editingMessageId ?? this.editingMessageId),
      isSending: isSending ?? this.isSending,
    );
  }
}

class ChatViewError extends ChatViewState {
  final String message;
  final List<ConversationMessage> messages;
  const ChatViewError(this.message, [this.messages = const []]);
}

/// Chat state notifier — manages message list, connection status, typing indicators,
/// and message editing for a single conversation.
class ChatNotifier extends StateNotifier<ChatViewState> {
  final ChatService _chatService;
  final UserService _userService;
  final String conversationId;

  Timer? _typingTimer;
  String? _currentUserId;
  String? _connectionStatus;

  ChatNotifier({
    required this.conversationId,
    ChatService? chatService,
    UserService? userService,
  }) : _chatService = chatService ?? ChatService(),
       _userService = userService ?? UserService(),
       super(const ChatViewInitial()) {
    _loadCurrentUser();
    loadMessages();
  }

  ChatViewState get currentState => state;

  static String? _normalizeConnectionStatus(String? status) {
    if (status == 'established') {
      return 'connected';
    }
    return status;
  }

  Future<void> _loadCurrentUser() async {
    try {
      final profile = await _userService.getUserProfile();
      _currentUserId = profile['user_id']?.toString();
      if (state is ChatViewData) {
        state = (state as ChatViewData).copyWith(currentUserId: _currentUserId);
      }
    } catch (_) {}
  }

  Future<void> hydrateConnectionStatus() async {
    try {
      final connections = await _chatService.getConnections();
      final conversation = connections
          .where((c) => c.id == conversationId)
          .firstOrNull;
      setConnectionStatus(conversation?.status);
    } catch (_) {
      // Best-effort hydrate. Live WS events and send paths still update state.
    }
  }

  Future<void> loadMessages() async {
    if (state is ChatViewData) {
      state = (state as ChatViewData).copyWith();
    } else {
      state = const ChatViewLoading();
    }
    try {
      final messages = await _chatService.getChatConversationMessages(
        conversationId,
      );
      final currentState = state;
      if (currentState is ChatViewData) {
        state = currentState.copyWith(
          messages: messages.reversed.toList(),
          currentUserId: _currentUserId,
          connectionStatus: _connectionStatus,
          isSending: false,
        );
      } else {
        state = ChatViewData(
          messages: messages.reversed.toList(),
          currentUserId: _currentUserId,
          connectionStatus: _connectionStatus,
        );
      }
      await _chatService.markConnectionAsRead(conversationId);
    } catch (e) {
      state = ChatViewError(e.toString());
    }
  }

  void setConnectionStatus(String? status) {
    _connectionStatus = _normalizeConnectionStatus(status);
    if (state is ChatViewData) {
      state = (state as ChatViewData).copyWith(
        connectionStatus: _connectionStatus,
      );
    }
  }

  void setOtherTyping(bool typing) {
    if (state is ChatViewData) {
      state = (state as ChatViewData).copyWith(isOtherTyping: typing);
    }
  }

  void startEditMessage(ConversationMessage msg) {
    if (state is ChatViewData) {
      state = (state as ChatViewData).copyWith(editingMessageId: msg.id);
    }
  }

  void cancelEdit() {
    if (state is ChatViewData) {
      state = (state as ChatViewData).copyWith(clearEditing: true);
    }
  }

  Future<void> confirmEdit(String newContent) async {
    if (state is! ChatViewData) return;
    final currentState = state as ChatViewData;
    if (currentState.editingMessageId == null) return;

    try {
      final updated = await _chatService.editMessage(
        currentState.editingMessageId!,
        newContent,
      );
      final idx = currentState.messages.indexWhere(
        (m) => m.id == currentState.editingMessageId,
      );
      final newMessages = List<ConversationMessage>.from(currentState.messages);
      if (idx >= 0) newMessages[idx] = updated;
      state = currentState.copyWith(messages: newMessages, clearEditing: true);
    } catch (e) {
      rethrow;
    }
  }

  Future<void> sendMessage({
    required String content,
    String? imageBase64,
    String? audioBase64,
    String? imageUrl,
    String? audioUrl,
  }) async {
    if (state is! ChatViewData) return;
    final currentState = state as ChatViewData;
    if (currentState.connectionStatus != 'connected') {
      throw Exception('等待连接建立后再发送消息');
    }

    final tempMsg = ConversationMessage(
      id: DateTime.now().millisecondsSinceEpoch.toString(),
      conversationId: conversationId,
      senderId: currentState.currentUserId ?? '',
      content: content,
      imageBase64: imageBase64,
      audioBase64: audioBase64,
      imageUrl: imageUrl,
      audioUrl: audioUrl,
      sentAt: DateTime.now(),
      status: 'sending',
    );

    state = currentState.copyWith(
      messages: [...currentState.messages, tempMsg],
      isSending: true,
    );

    try {
      final reply = await _chatService.sendMessage(
        conversationId,
        content: content,
        imageBase64: imageBase64,
        audioBase64: audioBase64,
        imageUrl: imageUrl,
        audioUrl: audioUrl,
      );
      if (state is ChatViewData) {
        final s = state as ChatViewData;
        final idx = s.messages.indexWhere((m) => m.id == tempMsg.id);
        final newMessages = List<ConversationMessage>.from(s.messages);
        if (idx >= 0) newMessages[idx] = reply;
        state = s.copyWith(messages: newMessages, isSending: false);
      }
    } catch (e) {
      if (state is ChatViewData) {
        final s = state as ChatViewData;
        state = s.copyWith(
          messages: s.messages.where((m) => m.id != tempMsg.id).toList(),
          isSending: false,
        );
      }
      rethrow;
    }
  }

  void sendTypingIndicator() {
    _chatService.sendTyping(conversationId).catchError((_) {});
  }

  Future<void> acceptConnection(String connectionId) async {
    await _chatService.acceptConnection(connectionId);
    setConnectionStatus('connected');
    await loadMessages();
  }

  Future<void> rejectConnection(String connectionId) async {
    await _chatService.rejectConnection(connectionId);
    setConnectionStatus('rejected');
  }

  void handleWsNotification(
    String eventType, {
    String? messageId,
    String? conversationId,
    String? typingUserId,
  }) {
    switch (eventType) {
      case 'connection_established':
        setConnectionStatus('connected');
        loadMessages();
        break;
      case 'connection_rejected':
        setConnectionStatus('rejected');
        break;
      case 'new_message':
        if (messageId != null) {
          _chatService.markMessageRead(messageId).catchError((_) {});
          loadMessages();
        }
        break;
      case 'message_read':
        loadMessages();
        break;
      case 'typing':
        if (state is! ChatViewData) return;
        final currentState = state as ChatViewData;
        if (conversationId == this.conversationId &&
            typingUserId != currentState.currentUserId) {
          state = currentState.copyWith(isOtherTyping: true);
          _typingTimer?.cancel();
          _typingTimer = Timer(const Duration(seconds: 3), () {
            if (state is ChatViewData) {
              state = (state as ChatViewData).copyWith(isOtherTyping: false);
            }
          });
        }
        break;
    }
  }

  @override
  void dispose() {
    _typingTimer?.cancel();
    super.dispose();
  }
}
