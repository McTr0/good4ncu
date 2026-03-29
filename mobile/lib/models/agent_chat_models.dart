/// Agent chat message — immutable, supports copyWith.
class AgentMessage {
  final String id;
  final String content;
  final bool isFromAgent;
  final DateTime timestamp;
  /// True while SSE stream is still delivering tokens (streaming indicator).
  final bool isPartial;

  const AgentMessage({
    required this.id,
    required this.content,
    required this.isFromAgent,
    required this.timestamp,
    this.isPartial = false,
  });

  AgentMessage copyWith({
    String? id,
    String? content,
    bool? isFromAgent,
    DateTime? timestamp,
    bool? isPartial,
  }) {
    return AgentMessage(
      id: id ?? this.id,
      content: content ?? this.content,
      isFromAgent: isFromAgent ?? this.isFromAgent,
      timestamp: timestamp ?? this.timestamp,
      isPartial: isPartial ?? this.isPartial,
    );
  }
}

/// Agent conversation session — holds conversation ID and message history.
class AgentConversationSession {
  final String conversationId;
  final List<AgentMessage> messages;

  const AgentConversationSession({
    required this.conversationId,
    required this.messages,
  });

  AgentConversationSession copyWith({
    String? conversationId,
    List<AgentMessage>? messages,
  }) {
    return AgentConversationSession(
      conversationId: conversationId ?? this.conversationId,
      messages: messages ?? this.messages,
    );
  }
}

/// Fixed conversation ID used for the agent chat session.
const String kAgentConversationId = '__agent__';
