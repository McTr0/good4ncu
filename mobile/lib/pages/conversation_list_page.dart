import 'dart:async';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import '../models/models.dart';
import '../services/api_service.dart';
import '../theme/app_theme.dart';

/// 会话列表页面
class ConversationListPage extends StatefulWidget {
  const ConversationListPage({super.key});

  @override
  State<ConversationListPage> createState() => _ConversationListPageState();
}

class _ConversationListPageState extends State<ConversationListPage> {
  final ApiService _apiService = ApiService();
  List<Conversation> _conversations = [];
  bool _isLoading = true;
  String? _error;

  StreamSubscription? _wsSubscription;

  @override
  void initState() {
    super.initState();
    _loadConversations();
  }

  @override
  void dispose() {
    _wsSubscription?.cancel();
    super.dispose();
  }

  Future<void> _loadConversations() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final connections = await _apiService.getConnections();
      if (!mounted) return;
      setState(() {
        _conversations = connections;
        _isLoading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('消息'),
        backgroundColor: AppTheme.primary,
        foregroundColor: Colors.white,
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text('加载失败: $_error'),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _loadConversations,
              child: const Text('重试'),
            ),
          ],
        ),
      );
    }
    if (_conversations.isEmpty) {
      return const Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.chat_bubble_outline, size: 64, color: Colors.grey),
            SizedBox(height: 16),
            Text('暂无消息', style: TextStyle(color: Colors.grey)),
          ],
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: _loadConversations,
      child: ListView.builder(
        itemCount: _conversations.length,
        itemBuilder: (context, index) {
          final conv = _conversations[index];
          return _ConversationTile(
            conversation: conv,
            onTap: () {
              context.push('/chat/${conv.id}', extra: {
                'conversationId': conv.id,
                'otherUserId': conv.otherUserId,
                'otherUsername': conv.otherUsername,
              });
            },
          );
        },
      ),
    );
  }
}

class _ConversationTile extends StatelessWidget {
  final Conversation conversation;
  final VoidCallback onTap;

  const _ConversationTile({
    required this.conversation,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Stack(
        children: [
          CircleAvatar(
            backgroundColor: AppTheme.primary.withValues(alpha: 0.1),
            child: Text(
              conversation.otherUsername.isNotEmpty
                  ? conversation.otherUsername[0].toUpperCase()
                  : '?',
              style: const TextStyle(
                color: AppTheme.primary,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
          Positioned(
            right: 0,
            bottom: 0,
            child: _StatusDot(status: conversation.connectionStatus),
          ),
        ],
      ),
      title: Row(
        children: [
          Expanded(
            child: Text(
              conversation.otherUsername,
              style: const TextStyle(fontWeight: FontWeight.w600),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          if (conversation.lastMessageAt != null)
            Text(
              _formatTime(conversation.lastMessageAt!),
              style: const TextStyle(
                fontSize: 12,
                color: AppTheme.textSecondary,
              ),
            ),
        ],
      ),
      subtitle: Row(
        children: [
          Expanded(
            child: Text(
              conversation.lastMessage ?? '',
              style: const TextStyle(color: AppTheme.textSecondary),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          _StatusBadge(status: conversation.connectionStatus),
        ],
      ),
      onTap: onTap,
    );
  }

  String _formatTime(DateTime dt) {
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inMinutes < 1) return '刚刚';
    if (diff.inHours < 1) return '${diff.inMinutes}分钟前';
    if (diff.inDays < 1) return '${diff.inHours}小时前';
    if (diff.inDays < 7) return '${diff.inDays}天前';
    return '${dt.month}/${dt.day}';
  }
}

class _StatusDot extends StatelessWidget {
  final ConnectionStatusType status;

  const _StatusDot({required this.status});

  @override
  Widget build(BuildContext context) {
    Color color;
    switch (status) {
      case ConnectionStatusType.online:
        color = AppTheme.success;
        break;
      case ConnectionStatusType.offline:
        color = Colors.grey;
        break;
      case ConnectionStatusType.pending:
        color = AppTheme.warning;
        break;
    }
    return Container(
      width: 12,
      height: 12,
      decoration: BoxDecoration(
        color: color,
        shape: BoxShape.circle,
        border: Border.all(color: Colors.white, width: 2),
      ),
    );
  }
}

class _StatusBadge extends StatelessWidget {
  final ConnectionStatusType status;

  const _StatusBadge({required this.status});

  @override
  Widget build(BuildContext context) {
    String label;
    Color color;

    switch (status) {
      case ConnectionStatusType.online:
        label = '在线';
        color = AppTheme.success;
        break;
      case ConnectionStatusType.offline:
        label = '离线';
        color = Colors.grey;
        break;
      case ConnectionStatusType.pending:
        label = '待接受';
        color = AppTheme.warning;
        break;
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withValues(alpha: 0.3)),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 12,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}
