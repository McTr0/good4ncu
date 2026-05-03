import 'dart:convert';

import 'package:flutter/material.dart';

import '../components/audio_message_player.dart';
import '../models/models.dart';
import '../services/ws_service.dart';
import '../theme/app_theme.dart';

Future<void> showUserChatConnectionRequestDialog({
  required BuildContext context,
  required WsNotification notification,
  required ValueChanged<String> onAccept,
  required ValueChanged<String> onReject,
}) {
  final connectionId = notification.connectionId;
  if (connectionId == null) {
    return Future.value();
  }

  return showDialog<void>(
    context: context,
    barrierDismissible: false,
    builder: (ctx) => AlertDialog(
      title: const Text('连接请求'),
      content: Text(
        '${notification.title}\n\n${notification.body}\n\n确认后将开启消息已读功能',
      ),
      actions: [
        TextButton(
          onPressed: () {
            Navigator.pop(ctx);
            onReject(connectionId);
          },
          child: const Text('拒绝'),
        ),
        ElevatedButton(
          onPressed: () {
            Navigator.pop(ctx);
            onAccept(connectionId);
          },
          child: const Text('接受'),
        ),
      ],
    ),
  );
}

class ConnectionIndicator extends StatefulWidget {
  final String? status;
  final bool isWsConnected;

  const ConnectionIndicator({
    super.key,
    this.status,
    this.isWsConnected = false,
  });

  @override
  State<ConnectionIndicator> createState() => ConnectionIndicatorState();
}

class ConnectionIndicatorState extends State<ConnectionIndicator>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 800),
    )..repeat(reverse: true);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    Color color;
    String label;
    Widget dot;

    if (!widget.isWsConnected) {
      color = Colors.grey;
      label = '离线';
      dot = Container(
        width: 8,
        height: 8,
        decoration: BoxDecoration(color: color, shape: BoxShape.circle),
      );
    } else {
      switch (widget.status) {
        case 'connected':
          color = AppTheme.success;
          label = '在线';
          dot = Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          );
          break;
        case 'pending':
          color = AppTheme.warning;
          label = '待接受';
          dot = Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          );
          break;
        case 'connecting':
          color = AppTheme.warning;
          label = '连接中...';
          dot = AnimatedBuilder(
            animation: _controller,
            builder: (context, child) => Container(
              width: 8,
              height: 8,
              decoration: BoxDecoration(
                color: color.withValues(alpha: 0.5 + 0.5 * _controller.value),
                shape: BoxShape.circle,
              ),
            ),
          );
          break;
        default:
          color = Colors.grey;
          label = '离线';
          dot = Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          );
          break;
      }
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        dot,
        const SizedBox(width: 4),
        Text(
          label,
          style: TextStyle(
            fontSize: 12,
            color: Colors.white.withValues(alpha: 0.8),
          ),
        ),
      ],
    );
  }
}

class MessageBubble extends StatelessWidget {
  final ConversationMessage message;
  final bool isMe;
  final bool isConnected;
  final VoidCallback? onEdit;

  const MessageBubble({
    super.key,
    required this.message,
    required this.isMe,
    required this.isConnected,
    this.onEdit,
  });

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: isMe ? Alignment.centerRight : Alignment.centerLeft,
      child: GestureDetector(
        onLongPress: onEdit,
        child: Container(
          constraints: BoxConstraints(
            maxWidth: MediaQuery.of(context).size.width * 0.75,
          ),
          margin: const EdgeInsets.symmetric(vertical: 4),
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: isMe ? AppTheme.primary : Colors.grey[200],
            borderRadius: BorderRadius.circular(16).copyWith(
              bottomRight: isMe
                  ? const Radius.circular(0)
                  : const Radius.circular(16),
              bottomLeft: !isMe
                  ? const Radius.circular(0)
                  : const Radius.circular(16),
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if ((message.imageUrl != null && message.imageUrl!.isNotEmpty) ||
                  (message.imageBase64 != null &&
                      message.imageBase64!.isNotEmpty))
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(8),
                    child:
                        message.imageUrl != null && message.imageUrl!.isNotEmpty
                        ? Image.network(
                            message.imageUrl!,
                            width: 200,
                            fit: BoxFit.cover,
                            errorBuilder: (context, error, stackTrace) {
                              if (message.imageBase64 != null &&
                                  message.imageBase64!.isNotEmpty) {
                                return Image.memory(
                                  base64Decode(message.imageBase64!),
                                  width: 200,
                                  fit: BoxFit.cover,
                                );
                              }
                              return const SizedBox.shrink();
                            },
                          )
                        : Image.memory(
                            base64Decode(message.imageBase64!),
                            width: 200,
                            fit: BoxFit.cover,
                          ),
                  ),
                ),
              if ((message.audioUrl != null && message.audioUrl!.isNotEmpty) ||
                  (message.audioBase64 != null &&
                      message.audioBase64!.isNotEmpty))
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: AudioMessagePlayer(
                    audioUrl: message.audioUrl,
                    audioBase64: message.audioBase64,
                    isMe: isMe,
                  ),
                ),
              Text(
                message.content,
                style: TextStyle(
                  color: isMe ? Colors.white : Colors.black87,
                  fontSize: 16,
                ),
              ),
              if (message.editedAt != null)
                Padding(
                  padding: const EdgeInsets.only(top: 2),
                  child: Text(
                    '（已编辑）',
                    style: TextStyle(
                      fontSize: 10,
                      color: isMe ? Colors.white60 : Colors.black38,
                      fontStyle: FontStyle.italic,
                    ),
                  ),
                ),
              const SizedBox(height: 4),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    _formatTime(message.sentAt),
                    style: TextStyle(
                      fontSize: 10,
                      color: isMe ? Colors.white70 : Colors.black45,
                    ),
                  ),
                  if (isMe && onEdit != null) ...[
                    const SizedBox(width: 4),
                    GestureDetector(
                      onTap: onEdit,
                      child: Text(
                        '编辑',
                        style: TextStyle(
                          fontSize: 10,
                          color: Colors.white60,
                          decoration: TextDecoration.underline,
                        ),
                      ),
                    ),
                  ],
                  if (isMe) ...[const SizedBox(width: 4), _buildStatus()],
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildStatus() {
    if (!isConnected) {
      return const SizedBox.shrink();
    }
    switch (message.status) {
      case 'sending':
        return const Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox(
              width: 8,
              height: 8,
              child: CircularProgressIndicator(
                strokeWidth: 1,
                color: Colors.white54,
              ),
            ),
            SizedBox(width: 2),
            Text('发送中', style: TextStyle(fontSize: 10, color: Colors.white54)),
          ],
        );
      case 'sent':
        return const Text(
          '已发送',
          style: TextStyle(fontSize: 10, color: Colors.white70),
        );
      case 'delivered':
        return const Text(
          '已送达',
          style: TextStyle(fontSize: 10, color: Colors.white70),
        );
      case 'read':
        return const Text(
          '已读',
          style: TextStyle(fontSize: 10, color: AppTheme.success),
        );
      case 'failed':
        return const Text(
          '发送失败',
          style: TextStyle(fontSize: 10, color: Colors.red),
        );
      default:
        return const SizedBox.shrink();
    }
  }

  String _formatTime(DateTime dt) {
    return '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }
}

class UserChatTypingBanner extends StatelessWidget {
  final String username;

  const UserChatTypingBanner({super.key, required this.username});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      color: Colors.grey[100],
      child: Row(
        children: [
          const SizedBox(
            width: 12,
            height: 12,
            child: CircularProgressIndicator(strokeWidth: 2),
          ),
          const SizedBox(width: 8),
          Text(
            '$username 正在输入...',
            style: const TextStyle(color: Colors.grey, fontSize: 12),
          ),
        ],
      ),
    );
  }
}

class UserChatMessageList extends StatelessWidget {
  final bool isLoading;
  final String? error;
  final List<ConversationMessage> messages;
  final String? currentUserId;
  final String? connectionStatus;
  final ScrollController scrollController;
  final VoidCallback onRetry;
  final ValueChanged<ConversationMessage> onEditMessage;

  const UserChatMessageList({
    super.key,
    required this.isLoading,
    required this.error,
    required this.messages,
    required this.currentUserId,
    required this.connectionStatus,
    required this.scrollController,
    required this.onRetry,
    required this.onEditMessage,
  });

  @override
  Widget build(BuildContext context) {
    if (isLoading && messages.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (error != null && messages.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text('加载失败: $error'),
            const SizedBox(height: 16),
            ElevatedButton(onPressed: onRetry, child: const Text('重试')),
          ],
        ),
      );
    }
    if (messages.isEmpty) {
      return const Center(
        child: Text('暂无消息，开始聊天吧', style: TextStyle(color: Colors.grey)),
      );
    }

    return ListView.builder(
      controller: scrollController,
      padding: const EdgeInsets.all(16),
      itemCount: messages.length,
      itemBuilder: (context, index) {
        final msg = messages[index];
        final isMe = msg.isFrom(currentUserId ?? '');
        return MessageBubble(
          message: msg,
          isMe: isMe,
          isConnected: connectionStatus == 'connected',
          onEdit: isMe && msg.canEdit ? () => onEditMessage(msg) : null,
        );
      },
    );
  }
}

class UserChatInputArea extends StatelessWidget {
  final String? connectionStatus;
  final bool isRecording;
  final int recordingSeconds;
  final bool isSending;
  final bool isEditing;
  final TextEditingController textController;
  final VoidCallback onPickImage;
  final VoidCallback onToggleRecording;
  final VoidCallback onCancelEdit;
  final ValueChanged<String> onChanged;
  final ValueChanged<String> onSubmitted;
  final VoidCallback onSend;

  const UserChatInputArea({
    super.key,
    required this.connectionStatus,
    required this.isRecording,
    required this.recordingSeconds,
    required this.isSending,
    required this.isEditing,
    required this.textController,
    required this.onPickImage,
    required this.onToggleRecording,
    required this.onCancelEdit,
    required this.onChanged,
    required this.onSubmitted,
    required this.onSend,
  });

  @override
  Widget build(BuildContext context) {
    if (connectionStatus != 'connected') {
      return Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: Colors.orange.shade50,
          border: Border(top: BorderSide(color: Colors.orange.shade200)),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.hourglass_empty,
              color: Colors.orange.shade700,
              size: 18,
            ),
            const SizedBox(width: 8),
            Text('等待对方接受连接', style: TextStyle(color: Colors.orange.shade700)),
          ],
        ),
      );
    }

    if (isRecording) {
      return Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: Colors.red.shade50,
          border: Border(top: BorderSide(color: Colors.red.shade200)),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.circle, color: Colors.red, size: 12),
            const SizedBox(width: 8),
            Text(
              '录音中 ${recordingSeconds}s / 60s',
              style: TextStyle(
                color: Colors.red.shade700,
                fontWeight: FontWeight.bold,
              ),
            ),
            const Spacer(),
            TextButton(onPressed: onToggleRecording, child: const Text('停止')),
          ],
        ),
      );
    }

    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: Theme.of(context).cardTheme.color,
        border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: SafeArea(
        child: Row(
          children: [
            IconButton(
              icon: const Icon(Icons.image),
              onPressed: isSending ? null : onPickImage,
            ),
            IconButton(
              icon: Icon(
                isRecording ? Icons.stop : Icons.mic,
                color: isRecording ? Colors.red : null,
              ),
              onPressed: onToggleRecording,
            ),
            if (isEditing)
              IconButton(
                icon: const Icon(Icons.close, color: Colors.grey),
                onPressed: onCancelEdit,
              ),
            Expanded(
              child: TextField(
                controller: textController,
                decoration: InputDecoration(
                  hintText: isEditing ? '编辑消息...' : '输入消息...',
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(24),
                  ),
                  contentPadding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 8,
                  ),
                ),
                onChanged: onChanged,
                onSubmitted: onSubmitted,
              ),
            ),
            const SizedBox(width: 8),
            IconButton(
              icon: Icon(
                isEditing ? Icons.check : Icons.send,
                color: isEditing ? Colors.green : AppTheme.primary,
              ),
              onPressed: onSend,
            ),
          ],
        ),
      ),
    );
  }
}
