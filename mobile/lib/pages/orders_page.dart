import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import '../l10n/app_localizations.dart';
import '../services/api_service.dart';
import '../theme/app_theme.dart';

class OrdersPage extends StatefulWidget {
  const OrdersPage({super.key});

  @override
  State<OrdersPage> createState() => _OrdersPageState();
}

class _OrdersPageState extends State<OrdersPage> {
  final ApiService _apiService = ApiService();
  List<Map<String, dynamic>> _orders = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() { _loading = true; _error = null; });
    try {
      final data = await _apiService.getOrders();
      final items = data['orders'] as List<dynamic>? ?? [];
      if (mounted) {
        setState(() {
          _orders = items.cast<Map<String, dynamic>>();
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) setState(() { _loading = false; _error = '$e'; });
    }
  }

  String _statusLabel(BuildContext context, String status) {
    switch (status) {
      case 'pending': return '待支付';
      case 'paid': return '已支付';
      case 'shipped': return '已发货';
      case 'completed': return '已完成';
      case 'cancelled': return '已取消';
      default: return status;
    }
  }

  Color _statusColor(String status) {
    switch (status) {
      case 'pending': return Colors.orange;
      case 'paid': return AppTheme.primary;
      case 'shipped': return Colors.blue;
      case 'completed': return AppTheme.success;
      case 'cancelled': return AppTheme.error;
      default: return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(
        title: Text(l.myOrders),
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.error_outline, size: 48, color: AppTheme.error),
            const SizedBox(height: 16),
            Text(_error!, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: _load,
              child: Text(AppLocalizations.of(context)!.retry),
            ),
          ],
        ),
      );
    }
    if (_orders.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.receipt_long_outlined,
              size: 64,
              color: AppTheme.textSecondary.withValues(alpha: 0.5),
            ),
            const SizedBox(height: 16),
            Text(
              '暂无订单',
              style: TextStyle(fontSize: 16, color: AppTheme.textSecondary.withValues(alpha: 0.7)),
            ),
          ],
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView.builder(
        padding: const EdgeInsets.all(AppTheme.sp16),
        itemCount: _orders.length,
        itemBuilder: (context, i) {
          final order = _orders[i];
          final status = order['status'] as String? ?? 'unknown';
          final price = (order['final_price'] as int? ?? 0) / 100.0;
          return Card(
            margin: const EdgeInsets.only(bottom: 12),
            child: InkWell(
              onTap: () => context.push('/order/${order['id']}'),
              borderRadius: BorderRadius.circular(AppTheme.radiusMd),
              child: Padding(
                padding: const EdgeInsets.all(AppTheme.sp16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            '订单 ${order['id'].toString().substring(0, 8)}...',
                            style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
                          ),
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                          decoration: BoxDecoration(
                            color: _statusColor(status).withValues(alpha: 0.1),
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: Text(
                            _statusLabel(context, status),
                            style: TextStyle(
                              color: _statusColor(status),
                              fontWeight: FontWeight.w600,
                              fontSize: 12,
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    Text(
                      '¥${price.toStringAsFixed(2)}',
                      style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold, color: AppTheme.primary),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '商品ID: ${order['listing_id'].toString().substring(0, 8)}...',
                      style: TextStyle(fontSize: 12, color: AppTheme.textSecondary.withValues(alpha: 0.7)),
                    ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}
