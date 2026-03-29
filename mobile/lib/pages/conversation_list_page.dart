import 'dart:async';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import '../models/models.dart';
import '../services/api_service.dart';
import '../services/ws_service.dart';
import '../theme/app_theme.dart';
import '../l10n/app_localizations.dart';

/// 会话列表页面 — also shows pending incoming connection requests with accept/reject.
class ConversationListPage extends StatefulWidget {
  const ConversationListPage({super.key});

  @override
  State<ConversationListPage> createState() => _ConversationListPageState();
}

class _ConversationListPageState extends State<ConversationListPage> {
  final ApiService _apiService = ApiService();
  String? _currentUserId;
  List<Conversation> _conversations = [];
  bool _isLoading = true;
  String? _error;

  StreamSubscription? _wsSubscription;

  @override
  void initState() {
    super.initState();
    _loadCurrentUser();
    _loadConversations();
    _connectWs();
  }

  @override
  void dispose() {
    // Only cancel the local subscription — the global WS singleton persists.
    _wsSubscription?.cancel();
    super.dispose();
  }

  Future<void> _loadCurrentUser() async {
    try {
      final profile = await _apiService.getUserProfile();
      if (!mounted) return;
      setState(() {
        _currentUserId = profile['user_id']?.toString();
      });
    } catch (_) {}
  }

  Future<void> _loadConversations() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final connections = await _apiService.getConnections();
      debugPrint('_loadConversations got ${connections.length} connections');
      for (final c in connections) {
        debugPrint('  conv: id=${c.id} status=${c.status} isReceiver=${c.isReceiver} canRespond=${c.canRespond} otherUsername=${c.otherUsername}');
      }
      if (!mounted) return;
      setState(() {
        _conversations = connections;
        _isLoading = false;
      });
    } catch (e) {
      if (!mounted) return;
      debugPrint('_loadConversations error: $e');
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  void _connectWs() {
    _wsSubscription = WsService.instance.stream.listen((notification) {
      if (!mounted) return;
      final event = notification.eventType;
      debugPrint('WS received event: $event');
      // Refresh list on connection status changes
      if (event == 'connection_established' ||
          event == 'connection_rejected' ||
          event == 'connection_request') {
        debugPrint('Refreshing conversations due to WS event: $event');
        _loadConversations();
      }
    });
  }

  Future<void> _acceptConnection(String connectionId) async {
    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    debugPrint('_acceptConnection called for connection: $connectionId');
    try {
      await _apiService.acceptConnection(connectionId);
      debugPrint('_acceptConnection API succeeded for: $connectionId');
      if (!mounted) return;
      messenger.showSnackBar(SnackBar(
        content: Text(l.connectionAccepted),
        backgroundColor: Colors.green,
      ));
      debugPrint('Calling _loadConversations after accept...');
      _loadConversations();
    } catch (e, st) {
      if (!mounted) return;
      debugPrint('Accept connection error: $e\n$st');
      messenger.showSnackBar(SnackBar(
        content: Text(l.operationFailed(e.toString())),
        backgroundColor: Colors.red,
      ));
    }
  }

  Future<void> _rejectConnection(String connectionId) async {
    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    try {
      await _apiService.rejectConnection(connectionId);
      if (!mounted) return;
      messenger.showSnackBar(SnackBar(
        content: Text(l.connectionRejected),
        backgroundColor: Colors.orange,
      ));
      _loadConversations();
    } catch (e, st) {
      if (!mounted) return;
      debugPrint('Reject connection error: $e\n$st');
      messenger.showSnackBar(SnackBar(
        content: Text(l.operationFailed(e.toString())),
        backgroundColor: Colors.red,
      ));
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('消息'),
        backgroundColor: AppTheme.primary,
        foregroundColor: Colors.white,
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadConversations,
          ),
        ],
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

    // Separate pending incoming requests from regular conversations
    final pendingIncoming =
        _conversations.where((c) => c.canRespond && _currentUserId != null).toList();
    final regularConversations = _conversations
        .where((c) => !c.canRespond || _currentUserId == null)
        .toList();
    debugPrint('_buildBody: $_conversations.length total, pendingIncoming=${pendingIncoming.length}, regular=${regularConversations.length}, currentUserId=$_currentUserId');

    return RefreshIndicator(
      onRefresh: _loadConversations,
      child: ListView(
        children: [
          // Pending incoming requests section
          if (pendingIncoming.isNotEmpty) ...[
            _SectionHeader(
              title: '待处理请求',
              count: pendingIncoming.length,
            ),
            ...pendingIncoming.map((conv) => _PendingRequestTile(
                  key: ValueKey(conv.id),
                  conversation: conv,
                  onAccept: () => _acceptConnection(conv.id),
                  onReject: () => _rejectConnection(conv.id),
                )),
            const Divider(height: 1),
          ],
          // Regular conversations
          if (regularConversations.isNotEmpty) ...[
            ...regularConversations.map((conv) => _ConversationTile(
                  key: ValueKey(conv.id),
                  conversation: conv,
                  onTap: () {
                    context.push('/chat/${conv.id}', extra: {
                      'conversationId': conv.id,
                      'otherUserId': conv.otherUserId,
                      'otherUsername': conv.otherUsername,
                    });
                  },
                )),
          ],
          if (regularConversations.isEmpty && pendingIncoming.isEmpty)
            const Padding(
              padding: EdgeInsets.all(32),
              child: Center(
                child: Text('暂无消息', style: TextStyle(color: Colors.grey)),
              ),
            ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  final int count;

  const _SectionHeader({required this.title, required this.count});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
      color: Colors.grey.shade100,
      child: Row(
        children: [
          Text(
            title,
            style: const TextStyle(
              fontWeight: FontWeight.bold,
              fontSize: 14,
              color: AppTheme.textSecondary,
            ),
          ),
          const SizedBox(width: 8),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: AppTheme.warning.withValues(alpha: 0.2),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Text(
              count.toString(),
              style: const TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.bold,
                color: AppTheme.warning,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _PendingRequestTile extends StatelessWidget {
  final Conversation conversation;
  final VoidCallback onAccept;
  final VoidCallback onReject;

  const _PendingRequestTile({
    super.key,
    required this.conversation,
    required this.onAccept,
    required this.onReject,
  });

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return ListTile(
      leading: CircleAvatar(
        backgroundColor: AppTheme.warning.withValues(alpha: 0.1),
        child: Text(
          conversation.otherUsername.isNotEmpty
              ? conversation.otherUsername[0].toUpperCase()
              : '?',
          style: const TextStyle(
            color: AppTheme.warning,
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
      title: Text(
        conversation.otherUsername,
        style: const TextStyle(fontWeight: FontWeight.w600),
      ),
      subtitle: const Text(
        '想要与你建立连接',
        style: TextStyle(color: AppTheme.textSecondary, fontSize: 13),
      ),
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            icon: const Icon(Icons.check_circle, color: AppTheme.success),
            tooltip: l?.connectionAccepted ?? '接受',
            onPressed: onAccept,
          ),
          IconButton(
            icon: const Icon(Icons.cancel, color: AppTheme.error),
            tooltip: l?.connectionRejected ?? '拒绝',
            onPressed: onReject,
          ),
        ],
      ),
      onTap: () {
        // Could navigate to their profile or show details
      },
    );
  }
}

class _ConversationTile extends StatelessWidget {
  final Conversation conversation;
  final VoidCallback onTap;

  const _ConversationTile({
    super.key,
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
        label = '已连接';
        color = Colors.blueGrey;
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
