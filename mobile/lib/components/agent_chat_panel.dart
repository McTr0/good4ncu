import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/agent_chat_models.dart';
import '../providers/agent_chat_notifier.dart';
import '../theme/app_theme.dart';

/// Agent chat panel — displayed as an overlay when the FAB is tapped.
class AgentChatPanel extends StatefulWidget {
  final VoidCallback onClose;

  const AgentChatPanel({super.key, required this.onClose});

  @override
  State<AgentChatPanel> createState() => _AgentChatPanelState();
}

class _AgentChatPanelState extends State<AgentChatPanel> {
  final TextEditingController _inputController = TextEditingController();
  final ScrollController _scrollController = ScrollController();
  final FocusNode _focusNode = FocusNode();
  int _lastMessageCount = 0;
  int _lastPartialLength = 0;

  @override
  void initState() {
    super.initState();
    _lastMessageCount = 0;
    _lastPartialLength = 0;
  }

  @override
  void dispose() {
    _inputController.dispose();
    _scrollController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _sendMessage(AgentChatNotifier notifier) {
    final text = _inputController.text.trim();
    if (text.isEmpty) return;
    _inputController.clear();
    notifier.sendMessage(text);
  }

  void _scrollToBottomIfNeeded(List<AgentMessage> messages, bool isStreaming) {
    if (messages.length != _lastMessageCount) {
      _lastMessageCount = messages.length;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_scrollController.hasClients) {
          _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
        }
      });
      if (messages.isNotEmpty) {
        _lastPartialLength = messages.last.content.length;
      }
      return;
    }

    if (!isStreaming || messages.isEmpty) {
      return;
    }

    final lastMessage = messages.last;
    if (!lastMessage.isFromAgent || !lastMessage.isPartial) {
      return;
    }

    final nextPartialLength = lastMessage.content.length;
    if (nextPartialLength == _lastPartialLength) {
      return;
    }
    _lastPartialLength = nextPartialLength;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<AgentChatNotifier>(
      builder: (context, notifier, _) {
        final state = notifier.state;

        final messages = state is AgentChatLoaded
            ? state.messages
            : <AgentMessage>[];
        final isStreaming = state is AgentChatLoaded
            ? state.isStreaming
            : false;
        _scrollToBottomIfNeeded(messages, isStreaming);

        return Container(
          width: 360,
          height: 520,
          constraints: BoxConstraints(
            maxWidth: MediaQuery.of(context).size.width - 32,
            maxHeight: MediaQuery.of(context).size.height - 120,
          ),
          decoration: BoxDecoration(
            color:
                Theme.of(context).cardTheme.color ??
                (Theme.of(context).brightness == Brightness.dark
                    ? AppTheme.cardDark
                    : AppTheme.cardLight),
            borderRadius: BorderRadius.circular(AppTheme.radiusLg),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withValues(alpha: 0.15),
                blurRadius: 20,
                offset: const Offset(0, 8),
              ),
            ],
          ),
          child: Column(
            children: [
              _buildHeader(context),
              Expanded(child: _buildMessageList(state)),
              _buildInputBar(context, notifier, state),
            ],
          ),
        );
      },
    );
  }

  Widget _buildHeader(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppTheme.sp16,
        vertical: AppTheme.sp12,
      ),
      decoration: const BoxDecoration(
        color: AppTheme.primary,
        borderRadius: BorderRadius.only(
          topLeft: Radius.circular(AppTheme.radiusLg),
          topRight: Radius.circular(AppTheme.radiusLg),
        ),
      ),
      child: Row(
        children: [
          const CircleAvatar(
            radius: 18,
            backgroundColor: Colors.white24,
            child: Icon(
              Icons.smart_toy_outlined,
              color: Colors.white,
              size: 20,
            ),
          ),
          const SizedBox(width: AppTheme.sp8),
          const Expanded(
            child: Text(
              '小帮',
              style: TextStyle(
                color: Colors.white,
                fontSize: 16,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, color: Colors.white70, size: 20),
            onPressed: widget.onClose,
            padding: EdgeInsets.zero,
            constraints: const BoxConstraints(),
          ),
        ],
      ),
    );
  }

  Widget _buildMessageList(AgentChatState state) {
    if (state is AgentChatInitial) {
      return const Center(
        child: Text(
          '正在初始化...',
          style: TextStyle(color: AppTheme.textSecondary),
        ),
      );
    }

    if (state is AgentChatError) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.error_outline, color: AppTheme.error, size: 40),
            const SizedBox(height: AppTheme.sp8),
            Text(
              state.message,
              style: const TextStyle(color: AppTheme.textSecondary),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      );
    }

    final messages = state is AgentChatLoaded
        ? state.messages
        : <AgentMessage>[];
    final isStreaming = state is AgentChatLoaded ? state.isStreaming : false;
    final error = state is AgentChatLoaded ? state.error : null;

    return Column(
      children: [
        Expanded(
          child: ListView.builder(
            controller: _scrollController,
            padding: const EdgeInsets.all(AppTheme.sp12),
            itemCount: messages.length + (isStreaming ? 1 : 0),
            itemBuilder: (context, i) {
              if (i >= messages.length) {
                return _buildTypingIndicator();
              }
              return _MessageBubble(
                key: ValueKey(messages[i].id),
                message: messages[i],
              );
            },
          ),
        ),
        if (error != null)
          Container(
            padding: const EdgeInsets.symmetric(
              horizontal: AppTheme.sp12,
              vertical: AppTheme.sp4,
            ),
            color: AppTheme.error.withValues(alpha: 0.1),
            child: Row(
              children: [
                const Icon(
                  Icons.error_outline,
                  color: AppTheme.error,
                  size: 14,
                ),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    error,
                    style: const TextStyle(color: AppTheme.error, fontSize: 12),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }

  Widget _buildTypingIndicator() {
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.only(bottom: AppTheme.sp8),
        padding: const EdgeInsets.symmetric(
          horizontal: AppTheme.sp12,
          vertical: AppTheme.sp8,
        ),
        decoration: BoxDecoration(
          color: Theme.of(context).brightness == Brightness.dark
              ? AppTheme.surfaceDark
              : const Color(0xFFE8EAF6),
          borderRadius: const BorderRadius.only(
            topLeft: Radius.circular(AppTheme.radiusMd),
            topRight: Radius.circular(AppTheme.radiusMd),
            bottomRight: Radius.circular(AppTheme.radiusMd),
            bottomLeft: Radius.circular(4),
          ),
        ),
        child: _TypingDots(),
      ),
    );
  }

  Widget _buildInputBar(
    BuildContext context,
    AgentChatNotifier notifier,
    AgentChatState state,
  ) {
    final isBusy =
        state is AgentChatInitial ||
        state is AgentChatLoading ||
        (state is AgentChatLoaded && state.isStreaming);
    return Container(
      padding: const EdgeInsets.all(AppTheme.sp12),
      decoration: BoxDecoration(
        border: Border(
          top: BorderSide(
            color: Theme.of(context).brightness == Brightness.dark
                ? AppTheme.borderDark
                : AppTheme.borderLight,
          ),
        ),
      ),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              controller: _inputController,
              focusNode: _focusNode,
              enabled: !isBusy,
              decoration: InputDecoration(
                hintText: '问小帮任何问题...',
                hintStyle: const TextStyle(color: AppTheme.textSecondary),
                contentPadding: const EdgeInsets.symmetric(
                  horizontal: AppTheme.sp12,
                  vertical: AppTheme.sp8,
                ),
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(AppTheme.radiusXl),
                  borderSide: BorderSide.none,
                ),
                filled: true,
                fillColor: Theme.of(context).brightness == Brightness.dark
                    ? AppTheme.surfaceDark
                    : AppTheme.surface,
              ),
              textInputAction: TextInputAction.send,
              onSubmitted: isBusy ? null : (_) => _sendMessage(notifier),
            ),
          ),
          const SizedBox(width: AppTheme.sp8),
          Material(
            color: AppTheme.primary,
            borderRadius: BorderRadius.circular(AppTheme.radiusXl),
            child: InkWell(
              borderRadius: BorderRadius.circular(AppTheme.radiusXl),
              onTap: isBusy ? null : () => _sendMessage(notifier),
              child: Container(
                padding: const EdgeInsets.all(AppTheme.sp8),
                child: const Icon(Icons.send, color: Colors.white, size: 20),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// Separate widget for message bubbles so each can have a stable ValueKey.
class _MessageBubble extends StatelessWidget {
  final AgentMessage message;

  const _MessageBubble({super.key, required this.message});

  @override
  Widget build(BuildContext context) {
    final isAgent = message.isFromAgent;
    return Align(
      alignment: isAgent ? Alignment.centerLeft : Alignment.centerRight,
      child: Container(
        margin: const EdgeInsets.only(bottom: AppTheme.sp8),
        constraints: const BoxConstraints(maxWidth: 260),
        padding: const EdgeInsets.symmetric(
          horizontal: AppTheme.sp12,
          vertical: AppTheme.sp8,
        ),
        decoration: BoxDecoration(
          color: isAgent
              ? (Theme.of(context).brightness == Brightness.dark
                    ? AppTheme.surfaceDark
                    : const Color(0xFFE8EAF6))
              : AppTheme.primary,
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(AppTheme.radiusMd),
            topRight: const Radius.circular(AppTheme.radiusMd),
            bottomLeft: isAgent
                ? const Radius.circular(4)
                : const Radius.circular(AppTheme.radiusMd),
            bottomRight: isAgent
                ? const Radius.circular(AppTheme.radiusMd)
                : const Radius.circular(4),
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              message.content,
              style: TextStyle(
                color: isAgent ? AppTheme.textPrimary : Colors.white,
                fontSize: 14,
              ),
            ),
            if (message.isPartial)
              const Padding(
                padding: EdgeInsets.only(top: 4),
                child: _TypingDots(),
              ),
          ],
        ),
      ),
    );
  }
}

/// Animated typing dots indicator.
class _TypingDots extends StatelessWidget {
  const _TypingDots();

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: List.generate(3, (i) {
        return TweenAnimationBuilder<double>(
          tween: Tween(begin: 0.0, end: 1.0),
          duration: Duration(milliseconds: 600 + i * 150),
          builder: (context, value, child) {
            return Container(
              margin: const EdgeInsets.symmetric(horizontal: 2),
              width: 6,
              height: 6,
              decoration: BoxDecoration(
                color: AppTheme.textSecondary.withValues(
                  alpha: 0.3 + 0.7 * value,
                ),
                shape: BoxShape.circle,
              ),
            );
          },
        );
      }),
    );
  }
}
