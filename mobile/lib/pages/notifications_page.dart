import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../l10n/app_localizations.dart';
import '../models/models.dart';
import '../services/notification_service.dart';
import '../theme/app_theme.dart';

class NotificationsPage extends StatefulWidget {
  final NotificationService? notificationService;

  const NotificationsPage({super.key, this.notificationService});

  @override
  State<NotificationsPage> createState() => _NotificationsPageState();
}

class _NotificationsPageState extends State<NotificationsPage> {
  static const int _limit = 20;

  late final NotificationService _notificationService;

  List<AppNotification> _items = [];
  bool _loading = true;
  bool _loadingMore = false;
  String? _error;
  String? _paginationError;
  int _total = 0;
  int _offset = 0;
  int _unreadCount = 0;
  int _activeRequestId = 0;

  @override
  void initState() {
    super.initState();
    _notificationService = widget.notificationService ?? NotificationService();
    _load(reset: true);
  }

  bool get _hasMore => _items.length < _total;

  Future<void> _load({bool reset = false, int? requestOffset}) async {
    final effectiveOffset = reset ? 0 : (requestOffset ?? _offset);
    final requestId = ++_activeRequestId;

    if (reset) {
      setState(() {
        _loading = true;
        _error = null;
        _paginationError = null;
      });
    }

    try {
      final response = await _notificationService.getNotifications(
        limit: _limit,
        offset: effectiveOffset,
        includeRead: true,
      );
      if (!mounted || requestId != _activeRequestId) return;

      setState(() {
        if (reset) {
          _items = response.items;
        } else {
          _items = [..._items, ...response.items];
        }
        _total = response.total;
        _unreadCount = response.unreadCount;
        _offset = effectiveOffset;
        _loading = false;
        _loadingMore = false;
        _paginationError = null;
      });
    } catch (e) {
      if (!mounted || requestId != _activeRequestId) return;
      setState(() {
        if (reset) {
          _error = e.toString();
        } else {
          _paginationError = e.toString();
        }
        _loading = false;
        _loadingMore = false;
      });
    }
  }

  Future<void> _onRefresh() async {
    await _load(reset: true);
  }

  void _loadMore() {
    if (_loading || _loadingMore || !_hasMore) return;
    final nextOffset = _items.length;
    setState(() {
      _loadingMore = true;
      _paginationError = null;
    });
    _load(requestOffset: nextOffset);
  }

  String _formatDate(String raw) {
    if (raw.isEmpty) return '';
    if (raw.length >= 16) return raw.substring(0, 16).replaceFirst('T', ' ');
    if (raw.length >= 10) return raw.substring(0, 10);
    return raw;
  }

  Future<void> _markAsRead(AppNotification item, {bool silent = false}) async {
    if (item.isRead) return;

    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);

    try {
      await _notificationService.markNotificationRead(item.id);
      if (!mounted) return;

      setState(() {
        _items = _items
            .map((n) => n.id == item.id ? n.copyWith(isRead: true) : n)
            .toList();
        if (_unreadCount > 0) {
          _unreadCount -= 1;
        }
      });
    } catch (e) {
      if (!mounted || silent) return;
      messenger.showSnackBar(
        SnackBar(content: Text(l.operationFailed(e.toString()))),
      );
    }
  }

  Future<void> _markAllRead() async {
    if (_unreadCount == 0) return;

    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);

    try {
      await _notificationService.markAllRead();
      if (!mounted) return;

      setState(() {
        _items = _items.map((n) => n.copyWith(isRead: true)).toList();
        _unreadCount = 0;
      });

      messenger.showSnackBar(SnackBar(content: Text(l.markAllReadSuccess)));
    } catch (e) {
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(content: Text(l.operationFailed(e.toString()))),
      );
    }
  }

  Future<void> _onTapNotification(AppNotification item) async {
    await _markAsRead(item, silent: true);
    if (!mounted) return;

    if (item.relatedListingId != null && item.relatedListingId!.isNotEmpty) {
      context.push('/listing/${item.relatedListingId}');
      return;
    }

    if (item.relatedOrderId != null && item.relatedOrderId!.isNotEmpty) {
      context.push('/orders/${item.relatedOrderId}');
    }
  }

  IconData _iconForEvent(String eventType) {
    if (eventType.contains('message') || eventType.contains('chat')) {
      return Icons.chat_bubble_outline;
    }
    if (eventType.contains('order')) {
      return Icons.receipt_long_outlined;
    }
    if (eventType.contains('negotiate') || eventType.contains('negotiation')) {
      return Icons.handshake_outlined;
    }
    return Icons.notifications_none;
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;

    return Scaffold(
      appBar: AppBar(
        title: Text(l.notificationsCenter),
        actions: [
          TextButton.icon(
            onPressed: _unreadCount == 0 ? null : _markAllRead,
            icon: const Icon(Icons.done_all, size: 18),
            label: Text(l.markAllRead),
          ),
          const SizedBox(width: AppTheme.sp8),
        ],
      ),
      body: _buildBody(l),
    );
  }

  Widget _buildBody(AppLocalizations l) {
    if (_loading && _items.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_error != null && _items.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(AppTheme.sp16),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.error_outline, size: 48, color: AppTheme.error),
              const SizedBox(height: AppTheme.sp16),
              Text(_error!, textAlign: TextAlign.center),
              const SizedBox(height: AppTheme.sp16),
              ElevatedButton(
                onPressed: () => _load(reset: true),
                child: Text(l.retry),
              ),
            ],
          ),
        ),
      );
    }

    if (_items.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.notifications_none,
              size: 64,
              color: AppTheme.textSecondary.withValues(alpha: 0.55),
            ),
            const SizedBox(height: AppTheme.sp16),
            Text(
              l.notificationsEmpty,
              style: const TextStyle(
                fontSize: 16,
                color: AppTheme.textSecondary,
              ),
            ),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: _onRefresh,
      child: ListView.separated(
        padding: const EdgeInsets.all(AppTheme.sp16),
        itemCount:
            _items.length + ((_hasMore || _paginationError != null) ? 1 : 0),
        separatorBuilder: (context, index) =>
            const SizedBox(height: AppTheme.sp12),
        itemBuilder: (context, index) {
          if (index >= _items.length) {
            if (_paginationError != null) {
              return Center(
                child: TextButton.icon(
                  onPressed: _loadMore,
                  icon: const Icon(Icons.refresh),
                  label: Text(l.retry),
                ),
              );
            }
            if (_loadingMore) {
              return const Center(
                child: Padding(
                  padding: EdgeInsets.all(AppTheme.sp16),
                  child: CircularProgressIndicator(),
                ),
              );
            }
            return Center(
              child: TextButton.icon(
                onPressed: _loadMore,
                icon: const Icon(Icons.expand_more),
                label: Text(l.loadMore),
              ),
            );
          }

          final item = _items[index];
          return _NotificationCard(
            item: item,
            icon: _iconForEvent(item.eventType),
            formattedDate: _formatDate(item.createdAt),
            onTap: () => _onTapNotification(item),
          );
        },
      ),
    );
  }
}

class _NotificationCard extends StatelessWidget {
  final AppNotification item;
  final IconData icon;
  final String formattedDate;
  final VoidCallback onTap;

  const _NotificationCard({
    required this.item,
    required this.icon,
    required this.formattedDate,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: EdgeInsets.zero,
      child: InkWell(
        borderRadius: BorderRadius.circular(AppTheme.radiusMd),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(AppTheme.sp14),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                padding: const EdgeInsets.all(AppTheme.sp12),
                decoration: BoxDecoration(
                  color: AppTheme.primary.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(AppTheme.radiusSm),
                ),
                child: Icon(icon, color: AppTheme.primary),
              ),
              const SizedBox(width: AppTheme.sp12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            item.title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              fontSize: 15,
                              fontWeight: item.isRead
                                  ? FontWeight.w500
                                  : FontWeight.w700,
                            ),
                          ),
                        ),
                        if (!item.isRead)
                          Container(
                            width: 8,
                            height: 8,
                            decoration: const BoxDecoration(
                              color: AppTheme.primary,
                              shape: BoxShape.circle,
                            ),
                          ),
                      ],
                    ),
                    const SizedBox(height: AppTheme.sp4),
                    Text(
                      item.body,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontSize: 13,
                        color: AppTheme.textSecondary,
                        height: 1.4,
                      ),
                    ),
                    const SizedBox(height: AppTheme.sp8),
                    Row(
                      children: [
                        Text(
                          formattedDate,
                          style: const TextStyle(
                            fontSize: 12,
                            color: AppTheme.textSecondary,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
