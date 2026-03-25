import 'package:flutter/material.dart';
import '../services/api_service.dart';
import '../theme/app_theme.dart';
import '../components/escrow_badge.dart';

/// Order detail page showing status machine + action buttons.
class OrderDetailPage extends StatefulWidget {
  final String orderId;

  const OrderDetailPage({super.key, required this.orderId});

  @override
  State<OrderDetailPage> createState() => _OrderDetailPageState();
}

class _OrderDetailPageState extends State<OrderDetailPage> {
  final ApiService _api = ApiService();
  Map<String, dynamic>? _order;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadOrder();
  }

  Future<void> _loadOrder() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final order = await _api.getOrder(widget.orderId);
      if (mounted) {
        setState(() {
          _order = order;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = '$e';
          _loading = false;
        });
      }
    }
  }

  Future<void> _doAction(Future<void> Function() action) async {
    try {
      await action();
      await _loadOrder();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('操作失败: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('订单详情')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text('加载失败: $_error'),
                      const SizedBox(height: 16),
                      ElevatedButton(
                        onPressed: _loadOrder,
                        child: const Text('重试'),
                      ),
                    ],
                  ),
                )
              : _buildContent(),
    );
  }

  Widget _buildContent() {
    final status = _order?['status'] as String? ?? 'unknown';
    final listingId = _order?['listing_id'] as String? ?? '';
    final finalPrice = (_order?['final_price'] as num?)?.toDouble() ?? 0.0;
    final buyerId = _order?['buyer_id'] as String? ?? '';
    final createdAt = _order?['created_at'] as String? ?? '';

    return SingleChildScrollView(
      padding: const EdgeInsets.all(AppTheme.sp16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Status card
          _StatusCard(status: status, finalPrice: finalPrice),
          const SizedBox(height: 16),

          // Escrow badge (for paid/pending)
          if (status == 'paid' || status == 'pending')
            EscrowBadge(amountCny: finalPrice / 100.0),

          const SizedBox(height: 24),

          // Order info
          _InfoSection(
            items: [
              _InfoItem(label: '商品', value: listingId),
              _InfoItem(label: '订单号', value: widget.orderId),
              _InfoItem(label: '订单金额', value: '¥${(finalPrice / 100).toStringAsFixed(2)}'),
              _InfoItem(label: '创建时间', value: _formatTime(createdAt)),
              _InfoItem(label: '状态', value: _statusLabel(status)),
            ],
          ),

          const SizedBox(height: 24),

          // Timeline
          _TimelineSection(order: _order!),

          const SizedBox(height: 24),

          // Action buttons
          _ActionButtons(
            status: status,
            orderId: widget.orderId,
            currentUserId: buyerId,
            onAction: _doAction,
            onRefresh: _loadOrder,
          ),
        ],
      ),
    );
  }

  String _statusLabel(String status) {
    switch (status) {
      case 'pending':
        return '待付款';
      case 'paid':
        return '已付款（托管中）';
      case 'shipped':
        return '已发货';
      case 'completed':
        return '已完成';
      case 'cancelled':
        return '已取消';
      default:
        return status;
    }
  }

  String _formatTime(String iso) {
    try {
      final dt = DateTime.parse(iso);
      return '${dt.year}/${dt.month}/${dt.day} ${dt.hour}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {
      return iso;
    }
  }
}

class _StatusCard extends StatelessWidget {
  final String status;
  final double finalPrice;

  const _StatusCard({required this.status, required this.finalPrice});

  Color get _color {
    switch (status) {
      case 'pending':
        return AppTheme.warning;
      case 'paid':
        return AppTheme.info;
      case 'shipped':
        return AppTheme.shipped;
      case 'completed':
        return AppTheme.success;
      case 'cancelled':
        return Colors.grey;
      default:
        return Colors.grey;
    }
  }

  IconData get _icon {
    switch (status) {
      case 'pending':
        return Icons.schedule;
      case 'paid':
        return Icons.account_balance_wallet;
      case 'shipped':
        return Icons.local_shipping;
      case 'completed':
        return Icons.check_circle;
      case 'cancelled':
        return Icons.cancel;
      default:
        return Icons.help;
    }
  }

  String get _label {
    switch (status) {
      case 'pending':
        return '等待买家付款';
      case 'paid':
        return '款项已托管，卖家待发货';
      case 'shipped':
        return '卖家已发货，等待确认收货';
      case 'completed':
        return '交易完成，卖家已收款';
      case 'cancelled':
        return '订单已取消';
      default:
        return '未知状态';
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(AppTheme.sp16),
      decoration: BoxDecoration(
        color: _color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(AppTheme.radiusMd),
        border: Border.all(color: _color.withValues(alpha: 0.3)),
      ),
      child: Row(
        children: [
          Icon(_icon, color: _color, size: 40),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _label,
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.bold,
                    color: _color,
                  ),
                ),
                if (status == 'pending') ...[
                  const SizedBox(height: 4),
                  Text(
                    '请在规定时间内完成付款',
                    style: TextStyle(fontSize: 12, color: _color.withValues(alpha: 0.8)),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _InfoSection extends StatelessWidget {
  final List<_InfoItem> items;

  const _InfoSection({required this.items});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          '订单信息',
          style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
        ),
        const SizedBox(height: 12),
        ...items.map((item) => Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  SizedBox(
                    width: 80,
                    child: Text(
                      item.label,
                      style: TextStyle(color: AppTheme.textSecondary, fontSize: 14),
                    ),
                  ),
                  Expanded(
                    child: Text(
                      item.value,
                      style: const TextStyle(fontSize: 14),
                    ),
                  ),
                ],
              ),
            )),
      ],
    );
  }
}

class _InfoItem {
  final String label;
  final String value;

  _InfoItem({required this.label, required this.value});
}

class _TimelineSection extends StatelessWidget {
  final Map<String, dynamic> order;

  const _TimelineSection({required this.order});

  @override
  Widget build(BuildContext context) {
    final steps = <_TimelineStep>[
      _TimelineStep(
        icon: Icons.receipt,
        label: '订单创建',
        time: order['created_at'] as String?,
        done: true,
      ),
      if (order['paid_at'] != null)
        _TimelineStep(
          icon: Icons.payment,
          label: '买家付款',
          time: order['paid_at'] as String?,
          done: true,
        ),
      if (order['shipped_at'] != null)
        _TimelineStep(
          icon: Icons.local_shipping,
          label: '卖家发货',
          time: order['shipped_at'] as String?,
          done: true,
        ),
      if (order['completed_at'] != null)
        _TimelineStep(
          icon: Icons.check_circle,
          label: '确认收货',
          time: order['completed_at'] as String?,
          done: true,
        ),
      if (order['cancelled_at'] != null)
        _TimelineStep(
          icon: Icons.cancel,
          label: '订单取消',
          time: order['cancelled_at'] as String?,
          done: true,
        ),
    ];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          '订单时间线',
          style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
        ),
        const SizedBox(height: 12),
        ...steps.map((step) => _TimelineItem(step: step)),
      ],
    );
  }
}

class _TimelineStep {
  final IconData icon;
  final String label;
  final String? time;
  final bool done;

  _TimelineStep({
    required this.icon,
    required this.label,
    this.time,
    required this.done,
  });
}

class _TimelineItem extends StatelessWidget {
  final _TimelineStep step;

  const _TimelineItem({required this.step});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Row(
        children: [
          Icon(step.icon, size: 20, color: step.done ? AppTheme.success : Colors.grey),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              step.label,
              style: TextStyle(
                color: step.done ? AppTheme.textPrimary : AppTheme.textSecondary,
                fontWeight: step.done ? FontWeight.w600 : FontWeight.normal,
              ),
            ),
          ),
          if (step.time != null)
            Text(
              _formatTime(step.time!),
              style: const TextStyle(fontSize: 12, color: AppTheme.textSecondary),
            ),
        ],
      ),
    );
  }

  String _formatTime(String iso) {
    try {
      final dt = DateTime.parse(iso);
      return '${dt.month}/${dt.day} ${dt.hour}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {
      return iso;
    }
  }
}

class _ActionButtons extends StatelessWidget {
  final String status;
  final String orderId;
  final String currentUserId;
  final Future<void> Function(Future<void> Function()) onAction;
  final VoidCallback onRefresh;

  const _ActionButtons({
    required this.status,
    required this.orderId,
    required this.currentUserId,
    required this.onAction,
    required this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    final api = ApiService();

    if (status == 'pending') {
      return SizedBox(
        width: double.infinity,
        child: ElevatedButton(
          style: ElevatedButton.styleFrom(backgroundColor: AppTheme.primary),
          onPressed: () => onAction(() => api.payOrder(orderId)),
          child: const Text('立即付款'),
        ),
      );
    }

    if (status == 'paid') {
      return Column(
        children: [
          SizedBox(
            width: double.infinity,
            child: ElevatedButton.icon(
              style: ElevatedButton.styleFrom(
                backgroundColor: AppTheme.shipped,
                foregroundColor: Colors.white,
              ),
              onPressed: () => onAction(() => api.shipOrder(orderId)),
              icon: const Icon(Icons.local_shipping, size: 18),
              label: const Text('确认发货'),
            ),
          ),
          const SizedBox(height: 12),
          SizedBox(
            width: double.infinity,
            child: OutlinedButton(
              style: OutlinedButton.styleFrom(foregroundColor: AppTheme.error),
              onPressed: () => _showCancelDialog(context, api),
              child: const Text('取消订单'),
            ),
          ),
        ],
      );
    }

    if (status == 'shipped') {
      return SizedBox(
        width: double.infinity,
        child: ElevatedButton(
          style: ElevatedButton.styleFrom(backgroundColor: AppTheme.success),
          onPressed: () => onAction(() => api.confirmOrder(orderId)),
          child: const Text('确认收货'),
        ),
      );
    }

    return const SizedBox.shrink();
  }

  void _showCancelDialog(BuildContext context, ApiService api) {
    final controller = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('取消订单'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(
            hintText: '取消原因（可选）',
            border: OutlineInputBorder(),
          ),
          maxLines: 2,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('返回'),
          ),
          TextButton(
            style: TextButton.styleFrom(foregroundColor: AppTheme.error),
            onPressed: () {
              Navigator.pop(ctx);
              onAction(() => api.cancelOrder(orderId, reason: controller.text));
            },
            child: const Text('确认取消'),
          ),
        ],
      ),
    );
  }
}
