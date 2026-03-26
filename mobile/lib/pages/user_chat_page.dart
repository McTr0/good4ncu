import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import '../models/models.dart';
import '../services/api_service.dart';
import '../services/ws_service.dart';
import '../theme/app_theme.dart';

/// 私聊页面
class UserChatPage extends StatefulWidget {
  final String conversationId;
  final String otherUserId;
  final String otherUsername;

  const UserChatPage({
    super.key,
    required this.conversationId,
    required this.otherUserId,
    required this.otherUsername,
  });

  @override
  State<UserChatPage> createState() => _UserChatPageState();
}

class _UserChatPageState extends State<UserChatPage> {
  final ApiService _apiService = ApiService();
  final TextEditingController _textController = TextEditingController();
  final ScrollController _scrollController = ScrollController();

  List<ConversationMessage> _messages = [];
  String? _currentUserId;
  bool _isLoading = true;
  String? _error;

  /// 连接状态: null=无连接, 'connecting'=连接中, 'connected'=已连接
  String? _connectionStatus;

  StreamSubscription? _wsSubscription;
  WsService? _wsService;

  @override
  void initState() {
    super.initState();
    _loadCurrentUser();
    _loadMessages();
    _connectWs();
  }

  @override
  void dispose() {
    _textController.dispose();
    _scrollController.dispose();
    _wsSubscription?.cancel();
    _wsService?.dispose();
    super.dispose();
  }

  Future<void> _loadCurrentUser() async {
    try {
      final profile = await _apiService.getUserProfile();
      setState(() {
        _currentUserId = profile['user_id']?.toString();
      });
    } catch (_) {}
  }

  Future<void> _loadMessages() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final messages = await _apiService.getChatConversationMessages(
        widget.conversationId,
      );
      if (!mounted) return;
      setState(() {
        _messages = messages.reversed.toList();
        _isLoading = false;
      });
      _scrollToBottom();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  Future<void> _connectWs() async {
    _wsService = WsService();
    try {
      await _wsService!.connect();
      _wsSubscription = _wsService!.stream.listen(_handleWsNotification);
    } catch (e) {
      debugPrint('WS connect failed: $e');
    }
  }

  void _handleWsNotification(WsNotification notif) {
    if (!mounted) return;

    switch (notif.eventType) {
      case 'connection_established':
        setState(() => _connectionStatus = 'connected');
        _loadMessages();
        _showSnackBar('连接已建立');
        break;

      case 'new_message':
        final messageId = notif.messageId;
        if (messageId != null) {
          _apiService.markMessageRead(messageId).catchError((_) {});
          _loadMessages();
        }
        break;

      case 'message_read':
        _loadMessages();
        break;

      case 'connection_request':
        _showConnectionRequestDialog(notif);
        break;
    }
  }

  void _showConnectionRequestDialog(WsNotification notif) {
    final connectionId = notif.connectionId;
    if (connectionId == null) return;

    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => AlertDialog(
        title: const Text('连接请求'),
        content: Text(
          '${notif.title}\n\n${notif.body}\n\n确认后将开启消息已读功能',
        ),
        actions: [
          TextButton(
            onPressed: () {
              Navigator.pop(ctx);
              _rejectConnection(connectionId);
            },
            child: const Text('拒绝'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.pop(ctx);
              _acceptConnection(connectionId);
            },
            child: const Text('接受'),
          ),
        ],
      ),
    );
  }

  Future<void> _acceptConnection(String connectionId) async {
    try {
      await _apiService.acceptConnection(connectionId);
      setState(() => _connectionStatus = 'connected');
      _showSnackBar('已接受连接');
      _loadMessages();
    } catch (e) {
      _showSnackBar('接受失败: $e');
    }
  }

  Future<void> _rejectConnection(String connectionId) async {
    try {
      await _apiService.rejectConnection(connectionId);
      _showSnackBar('已拒绝连接');
    } catch (e) {
      _showSnackBar('拒绝失败: $e');
    }
  }

  Future<void> _sendMessage() async {
    if (_connectionStatus != 'connected') {
      _showSnackBar('等待连接建立后再发送消息');
      return;
    }
    final text = _textController.text.trim();
    if (text.isEmpty) return;

    _textController.clear();
    setState(() => _isLoading = true);

    try {
      final message = await _apiService.sendMessage(
        widget.conversationId,
        content: text,
      );
      if (!mounted) return;
      setState(() {
        _messages.add(message);
        _isLoading = false;
      });
      _scrollToBottom();
    } catch (e) {
      if (!mounted) return;
      setState(() => _isLoading = false);
      _showSnackBar('发送失败: $e');
    }
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 300),
          curve: Curves.easeOut,
        );
      }
    });
  }

  void _showSnackBar(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), duration: const Duration(seconds: 2)),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: AppTheme.primary,
        foregroundColor: Colors.white,
        title: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            _ConnectionIndicator(status: _connectionStatus),
            const SizedBox(width: 8),
            Text(widget.otherUsername),
          ],
        ),
      ),
      body: Column(
        children: [
          Expanded(child: _buildMessageList()),
          _buildInputArea(),
        ],
      ),
    );
  }

  Widget _buildMessageList() {
    if (_isLoading && _messages.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null && _messages.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text('加载失败: $_error'),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _loadMessages,
              child: const Text('重试'),
            ),
          ],
        ),
      );
    }
    if (_messages.isEmpty) {
      return const Center(
        child: Text('暂无消息，开始聊天吧', style: TextStyle(color: Colors.grey)),
      );
    }
    return ListView.builder(
      controller: _scrollController,
      padding: const EdgeInsets.all(16),
      itemCount: _messages.length,
      itemBuilder: (context, index) {
        final msg = _messages[index];
        final isMe = msg.isFrom(_currentUserId ?? '');
        return _MessageBubble(
          message: msg,
          isMe: isMe,
          isConnected: _connectionStatus == 'connected',
        );
      },
    );
  }

  Widget _buildInputArea() {
    if (_connectionStatus != 'connected') {
      return Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: Colors.orange.shade50,
          border: Border(top: BorderSide(color: Colors.orange.shade200)),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.hourglass_empty, color: Colors.orange.shade700, size: 18),
            const SizedBox(width: 8),
            Text(
              '等待对方接受连接',
              style: TextStyle(color: Colors.orange.shade700),
            ),
          ],
        ),
      );
    }
    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: Theme.of(context).cardTheme.color,
        border: Border(
          top: BorderSide(color: Theme.of(context).dividerColor),
        ),
      ),
      child: SafeArea(
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: _textController,
                decoration: InputDecoration(
                  hintText: '输入消息...',
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(24),
                  ),
                  contentPadding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 8,
                  ),
                ),
                onSubmitted: (_) => _sendMessage(),
              ),
            ),
            const SizedBox(width: 8),
            IconButton(
              icon: const Icon(Icons.send),
              color: AppTheme.primary,
              onPressed: _sendMessage,
            ),
          ],
        ),
      ),
    );
  }
}

class _ConnectionIndicator extends StatefulWidget {
  final String? status;

  const _ConnectionIndicator({this.status});

  @override
  State<_ConnectionIndicator> createState() => _ConnectionIndicatorState();
}

class _ConnectionIndicatorState extends State<_ConnectionIndicator>
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

    switch (widget.status) {
      case 'connected':
        color = AppTheme.success;
        label = '在线';
        dot = Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(
            color: color,
            shape: BoxShape.circle,
          ),
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
          decoration: BoxDecoration(
            color: color,
            shape: BoxShape.circle,
          ),
        );
        break;
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

class _MessageBubble extends StatelessWidget {
  final ConversationMessage message;
  final bool isMe;
  final bool isConnected;

  const _MessageBubble({
    required this.message,
    required this.isMe,
    required this.isConnected,
  });

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: isMe ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        constraints: BoxConstraints(
          maxWidth: MediaQuery.of(context).size.width * 0.75,
        ),
        margin: const EdgeInsets.symmetric(vertical: 4),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: isMe ? AppTheme.primary : Colors.grey[200],
          borderRadius: BorderRadius.circular(16).copyWith(
            bottomRight: isMe ? const Radius.circular(0) : const Radius.circular(16),
            bottomLeft: !isMe ? const Radius.circular(0) : const Radius.circular(16),
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (message.imageBase64 != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(8),
                  child: Image.memory(
                    base64Decode(message.imageBase64!),
                    width: 200,
                    fit: BoxFit.cover,
                  ),
                ),
              ),
            if (message.audioBase64 != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.mic,
                      size: 16,
                      color: isMe ? Colors.white : Colors.black87,
                    ),
                    const SizedBox(width: 4),
                    Text(
                      '语音消息',
                      style: TextStyle(
                        fontSize: 12,
                        color: isMe ? Colors.white70 : Colors.black54,
                      ),
                    ),
                  ],
                ),
              ),
            Text(
              message.content,
              style: TextStyle(
                color: isMe ? Colors.white : Colors.black87,
                fontSize: 16,
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
                if (isMe) ...[
                  const SizedBox(width: 4),
                  _buildStatus(),
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildStatus() {
    if (!isConnected) {
      return const SizedBox.shrink();
    }
    if (message.isRead) {
      return const Text(
        '已读',
        style: TextStyle(fontSize: 10, color: AppTheme.success),
      );
    }
    return const Text(
      '已送达',
      style: TextStyle(fontSize: 10, color: Colors.grey),
    );
  }

  String _formatTime(DateTime dt) {
    return '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }
}
