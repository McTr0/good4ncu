import 'dart:async';
import 'package:flutter/foundation.dart';
import '../models/agent_chat_models.dart';
import '../services/chat_service.dart';
import '../services/sse_service.dart';
import '../services/user_service.dart';

/// Sealed state for agent chat — mutually exclusive.
sealed class AgentChatState {
  const AgentChatState();
}

class AgentChatInitial extends AgentChatState {
  const AgentChatInitial();
}

/// Connecting to SSE stream.
class AgentChatLoading extends AgentChatState {
  const AgentChatLoading();
}

/// Chat loaded and ready.
class AgentChatLoaded extends AgentChatState {
  final List<AgentMessage> messages;

  /// True while SSE stream is still delivering tokens.
  final bool isStreaming;
  final String? error;

  const AgentChatLoaded({
    required this.messages,
    this.isStreaming = false,
    this.error,
  });

  AgentChatLoaded copyWith({
    List<AgentMessage>? messages,
    bool? isStreaming,
    String? error,
    bool clearError = false,
  }) {
    return AgentChatLoaded(
      messages: messages ?? this.messages,
      isStreaming: isStreaming ?? this.isStreaming,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Unrecoverable error state.
class AgentChatError extends AgentChatState {
  final String message;
  const AgentChatError(this.message);
}

/// Agent chat notifier — manages SSE streaming lifecycle for the "小帮" agent.
/// Uses the fixed conversation ID `kAgentConversationId`.
///
/// Key behaviors:
/// - Complete messages (greeting, direct responses) have `isComplete=true` and
///   are finalized immediately without waiting for stream close.
/// - Streaming tokens accumulate in a partial bubble until stream closes.
/// - Backend errors from SSE `{"error": "..."}` events are surfaced in the UI.
class AgentChatNotifier extends ChangeNotifier {
  final SseService _sseService;
  final ChatService _chatService;
  final UserService _userService;
  final String _conversationId;
  StreamSubscription<SseToken>? _sseSubscription;

  /// Partial message being accumulated during streaming.
  String _partialContent = '';
  int _activeStreamId = 0;
  bool _isDisposed = false;

  AgentChatState _state = const AgentChatInitial();

  AgentChatNotifier({
    SseService? sseService,
    ChatService? chatService,
    UserService? userService,
  }) : _sseService = sseService ?? SseService(),
       _chatService = chatService ?? ChatService(),
       _userService = userService ?? UserService(),
       _conversationId = kAgentConversationId;

  AgentChatState get state => _state;

  bool get hasMessages {
    if (_state is! AgentChatLoaded) {
      return false;
    }
    return (_state as AgentChatLoaded).messages.isNotEmpty;
  }

  void _setState(AgentChatState newState) {
    if (_isDisposed) return;
    _state = newState;
    notifyListeners();
  }

  Future<void> loadHistory() async {
    if (_isDisposed) return;
    final historyLoadStreamId = _activeStreamId;
    try {
      final history = await _chatService.getChatConversationMessages(
        _conversationId,
      );
      if (history.isEmpty) return;
      final profile = await _userService.getUserProfile();
      if (_isDisposed || historyLoadStreamId != _activeStreamId) {
        return;
      }
      final currentUserId = profile['user_id']?.toString();
      final agentMessages = <AgentMessage>[];
      for (final m in history) {
        final normalizedContent = m.content.trim();
        if (normalizedContent.isEmpty) {
          continue;
        }
        agentMessages.add(
          AgentMessage(
            id: m.id,
            content: normalizedContent,
            isFromAgent: m.senderId != currentUserId,
            timestamp: m.sentAt,
            isPartial: false,
          ),
        );
      }
      if (agentMessages.isEmpty) {
        return;
      }
      if (_state is AgentChatLoaded) {
        final currentState = _state as AgentChatLoaded;
        if (currentState.isStreaming || currentState.messages.isNotEmpty) {
          return;
        }
        _setState(currentState.copyWith(messages: agentMessages));
      } else {
        _setState(AgentChatLoaded(messages: agentMessages));
      }
    } catch (_) {
      // History load is best-effort — don't fail the initial state.
    }
  }

  /// Send a user-authored message to the agent and stream the response.
  Future<void> sendMessage(String text) async {
    final trimmed = text.trim();
    if (trimmed.isEmpty) return;
    await _sendMessageInternal(trimmed, addUserMessage: true);
  }

  /// Request an assistant greeting when there is no history.
  Future<void> requestGreeting() {
    return _sendMessageInternal('', addUserMessage: false);
  }

  Future<void> _sendMessageInternal(
    String text, {
    required bool addUserMessage,
  }) async {
    if (_isDisposed) return;
    if (_state is AgentChatLoading) return;
    if (_state is AgentChatLoaded && (_state as AgentChatLoaded).isStreaming) {
      return;
    }

    _activeStreamId += 1;
    final streamId = _activeStreamId;

    await _sseSubscription?.cancel();
    _sseSubscription = null;
    _sseService.disconnect();

    final currentMessages = _state is AgentChatLoaded
        ? (_state as AgentChatLoaded).messages
        : <AgentMessage>[];

    final nextMessages = addUserMessage
        ? [
            ...currentMessages,
            AgentMessage(
              id: DateTime.now().millisecondsSinceEpoch.toString(),
              content: text,
              isFromAgent: false,
              timestamp: DateTime.now(),
              isPartial: false,
            ),
          ]
        : currentMessages;

    _setState(
      AgentChatLoaded(messages: nextMessages, isStreaming: true, error: null),
    );

    try {
      _partialContent = '';
      await _sseService
          .connect(message: text, conversationId: _conversationId)
          .timeout(const Duration(seconds: 20));

      if (_isDisposed || streamId != _activeStreamId) {
        _sseService.disconnect();
        return;
      }

      _sseSubscription = _sseService.stream.listen(
        (token) => _onSseToken(streamId, token),
        onError: (error) => _onSseError(streamId, error),
        onDone: () => _onSseDone(streamId),
      );
      // No timeout needed — stream's onDone fires when connection closes naturally.
    } catch (e) {
      if (_isDisposed || streamId != _activeStreamId) {
        return;
      }
      _sseService.disconnect();
      _setState(
        AgentChatLoaded(
          messages: nextMessages,
          isStreaming: false,
          error: e.toString(),
        ),
      );
    }
  }

  void _onSseToken(int streamId, SseToken token) {
    if (_isDisposed) return;
    if (streamId != _activeStreamId) return;

    // Handle backend error events.
    if (token.error != null) {
      _setError(token.error!);
      return;
    }

    // Skip empty tokens (keep-alive, etc.).
    if (token.token.isEmpty) return;

    if (_state is! AgentChatLoaded) return;
    final currentState = _state as AgentChatLoaded;

    // Complete messages (greeting, direct response) — finalize immediately.
    // No partial bubble needed; treat as a finished message.
    if (token.isComplete) {
      final agentMsg = AgentMessage(
        id: DateTime.now().millisecondsSinceEpoch.toString(),
        content: token.token,
        isFromAgent: true,
        timestamp: DateTime.now(),
        isPartial: false,
      );
      _setState(
        currentState.copyWith(
          messages: [...currentState.messages, agentMsg],
          isStreaming: false,
        ),
      );
      return;
    }

    // Streaming token — accumulate in partial bubble.
    _partialContent += token.token;
    _addOrUpdatePartialMessage(currentState);
  }

  void _addOrUpdatePartialMessage(AgentChatLoaded currentState) {
    final lastMsg = currentState.messages.isNotEmpty
        ? currentState.messages.last
        : null;

    List<AgentMessage> newMessages;
    if (lastMsg != null && lastMsg.isFromAgent && lastMsg.isPartial) {
      // Update existing partial message.
      newMessages = [
        ...currentState.messages.sublist(0, currentState.messages.length - 1),
        lastMsg.copyWith(content: _partialContent),
      ];
    } else {
      // New partial agent message.
      final agentMsg = AgentMessage(
        id: DateTime.now().millisecondsSinceEpoch.toString(),
        content: _partialContent,
        isFromAgent: true,
        timestamp: DateTime.now(),
        isPartial: true,
      );
      newMessages = [...currentState.messages, agentMsg];
    }

    _setState(currentState.copyWith(messages: newMessages, isStreaming: true));
  }

  void _setError(String errorMessage) {
    _sseSubscription?.cancel();
    _sseSubscription = null;
    _sseService.disconnect();

    if (_state is AgentChatLoaded) {
      final currentState = _state as AgentChatLoaded;
      final newMessages = _finalizePartialMessages(currentState.messages);
      _setState(
        currentState.copyWith(
          messages: newMessages,
          isStreaming: false,
          error: errorMessage,
        ),
      );
    }
  }

  void _onSseError(int streamId, Object error) {
    if (_isDisposed) return;
    if (streamId != _activeStreamId) return;
    _sseSubscription?.cancel();
    _sseSubscription = null;
    _sseService.disconnect();

    if (_state is AgentChatLoaded) {
      final currentState = _state as AgentChatLoaded;
      final newMessages = _finalizePartialMessages(currentState.messages);
      _setState(
        currentState.copyWith(
          messages: newMessages,
          isStreaming: false,
          error: error.toString(),
        ),
      );
    }
  }

  void _onSseDone(int streamId) {
    if (_isDisposed) return;
    if (streamId != _activeStreamId) return;
    _sseSubscription?.cancel();
    _sseSubscription = null;
    _sseService.disconnect();

    if (_state is AgentChatLoaded) {
      final currentState = _state as AgentChatLoaded;
      final newMessages = _finalizePartialMessages(currentState.messages);
      _setState(
        currentState.copyWith(messages: newMessages, isStreaming: false),
      );
    }
  }

  List<AgentMessage> _finalizePartialMessages(List<AgentMessage> messages) {
    return messages.map((message) {
      if (message.isPartial) {
        return message.copyWith(isPartial: false);
      }
      return message;
    }).toList();
  }

  /// Close the panel — disconnect SSE but keep state.
  void closePanel() {
    if (_isDisposed) return;
    _activeStreamId += 1;
    _sseSubscription?.cancel();
    _sseSubscription = null;
    _sseService.disconnect();

    if (_state is AgentChatLoaded) {
      final currentState = _state as AgentChatLoaded;
      final hasPartialMessage = currentState.messages.any((m) => m.isPartial);
      final finalizedMessages = _finalizePartialMessages(currentState.messages);
      if (currentState.isStreaming || hasPartialMessage) {
        _setState(
          currentState.copyWith(
            messages: finalizedMessages,
            isStreaming: false,
          ),
        );
      }
    }
  }

  /// Clear any error shown in the UI.
  void clearError() {
    if (_state is AgentChatLoaded) {
      _setState((_state as AgentChatLoaded).copyWith(clearError: true));
    }
  }

  @override
  void dispose() {
    _isDisposed = true;
    _activeStreamId += 1;
    _sseSubscription?.cancel();
    _sseService.dispose();
    super.dispose();
  }
}
