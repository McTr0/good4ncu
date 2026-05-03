import 'package:flutter/material.dart';

import '../../l10n/app_localizations.dart';
import '../../services/api_service.dart';
import '../../theme/app_theme.dart';

class AdminOrdersTab extends StatefulWidget {
  final ApiService apiService;

  const AdminOrdersTab({super.key, required this.apiService});

  @override
  State<AdminOrdersTab> createState() => _AdminOrdersTabState();
}

class _AdminOrdersTabState extends State<AdminOrdersTab> {
  Map<String, dynamic>? _orders;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final data = await widget.apiService.getAdminOrders(limit: 50);
      setState(() {
        _orders = data;
        _loading = false;
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  Color _statusColor(String? s) {
    return switch (s) {
      'paid' => AppTheme.info,
      'shipped' => AppTheme.shipped,
      'confirmed' || 'completed' => AppTheme.success,
      'cancelled' => AppTheme.error,
      _ => AppTheme.textSecondary,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) return Center(child: Text('${l.error}: $_error'));

    final items = _orders?['orders'] as List? ?? [];

    return RefreshIndicator(
      onRefresh: _load,
      child: ListView.builder(
        itemCount: items.length,
        itemBuilder: (context, i) {
          final item = items[i];
          return ListTile(
            leading: CircleAvatar(
              backgroundColor: _statusColor(item['status']),
              child: Text(
                (item['status'] ?? '?')[0].toUpperCase(),
                style: const TextStyle(color: Colors.white, fontSize: 12),
              ),
            ),
            title: Text(
              l.orderNumber(
                (item['id'] ?? '').toString().substring(
                  0,
                  (item['id'] ?? '').toString().length.clamp(0, 8),
                ),
              ),
            ),
            subtitle: Text(
              '${item['status'] ?? l.unknown} · ¥${((item['final_price'] as num?)?.toDouble() ?? 0) / 100}',
            ),
            trailing: const Icon(
              Icons.chevron_right,
              color: AppTheme.textSecondary,
            ),
            onTap: () => _showOrderDetail(context, item),
          );
        },
      ),
    );
  }

  void _showOrderDetail(BuildContext context, Map<String, dynamic> item) {
    final l = AppLocalizations.of(context)!;
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => DraggableScrollableSheet(
        initialChildSize: 0.6,
        minChildSize: 0.3,
        maxChildSize: 0.9,
        expand: false,
        builder: (_, scrollController) => ListView(
          controller: scrollController,
          padding: const EdgeInsets.all(AppTheme.sp16),
          children: [
            Text(
              l.orderNumber(item['id'] ?? ''),
              style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold),
            ),
            const Divider(),
            ...item.entries.map(
              (e) => ListTile(
                dense: true,
                title: Text(e.key),
                trailing: Text(
                  e.key == 'final_price'
                      ? '${((e.value as num?)?.toDouble() ?? 0) / 100}'
                      : '${e.value}',
                ),
              ),
            ),
            const Divider(),
            Row(
              children: [
                Expanded(
                  child: OutlinedButton(
                    onPressed: () async {
                      try {
                        await widget.apiService.updateAdminOrderStatus(
                          item['id'],
                          'cancelled',
                        );
                        if (ctx.mounted) Navigator.pop(ctx);
                        _load();
                      } catch (e) {
                        if (ctx.mounted) {
                          ScaffoldMessenger.of(ctx).showSnackBar(
                            SnackBar(
                              content: Text(l.operationFailed(e.toString())),
                            ),
                          );
                        }
                      }
                    },
                    child: Text(l.cancel),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: FilledButton(
                    onPressed: () async {
                      try {
                        await widget.apiService.updateAdminOrderStatus(
                          item['id'],
                          'completed',
                        );
                        if (ctx.mounted) Navigator.pop(ctx);
                        _load();
                      } catch (e) {
                        if (ctx.mounted) {
                          ScaffoldMessenger.of(ctx).showSnackBar(
                            SnackBar(
                              content: Text(l.operationFailed(e.toString())),
                            ),
                          );
                        }
                      }
                    },
                    child: Text(l.confirm),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
