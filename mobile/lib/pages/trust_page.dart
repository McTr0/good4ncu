import 'package:flutter/material.dart';
import '../theme/app_theme.dart';
import '../components/escrow_badge.dart';

/// Trust & safety explanation page — transaction protection mechanism.
class TrustPage extends StatelessWidget {
  const TrustPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('交易保障')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(AppTheme.sp16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Hero escrow badge
            const EscrowBadge(amountCny: null),
            const SizedBox(height: 8),
            const Text(
              '所有交易均受平台交易保障机制保护',
              style: TextStyle(color: AppTheme.textSecondary, fontSize: 14),
            ),

            const SizedBox(height: 32),

            // How it works
            const Text(
              '交易保障机制',
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 16),

            _TrustSection(
              icon: Icons.account_balance_wallet,
              iconColor: AppTheme.info,
              title: '平台托管',
              subtitle: '买家付款后，款项由平台临时托管',
              description:
                  '买家付款后，资金不会立即到达卖家账户，而是由平台代为托管。此期间卖家可以看到订单状态，但无法提现。',
            ),
            const SizedBox(height: 12),

            _TrustSection(
              icon: Icons.local_shipping,
              iconColor: AppTheme.shipped,
              title: '发货确认',
              subtitle: '卖家发货后，买家有 7 天确认收货时间',
              description:
                  '卖家发货后，买家需在 7 天内确认收货。如逾期未确认，系统将自动确认收货并放款给卖家，保障卖家权益。',
            ),
            const SizedBox(height: 12),

            _TrustSection(
              icon: Icons.undo,
              iconColor: AppTheme.warning,
              title: '取消退款',
              subtitle: '订单取消后，款项自动退回买家',
              description:
                  '如交易取消（卖家未发货前取消、双方协商取消等），托管款项将自动原路退回买家支付账户，退款即时到账。',
            ),
            const SizedBox(height: 12),

            _TrustSection(
              icon: Icons.support_agent,
              iconColor: AppTheme.primary,
              title: '争议处理',
              subtitle: 'AI Agent 辅助纠纷调解',
              description:
                  '如买卖双方对商品状态有争议，AI Agent 将介入辅助调解。平台会根据聊天记录、商品描述、图片等信息给出调解建议。必要时人工客服介入仲裁。',
            ),

            const SizedBox(height: 32),

            // Disclaimer
            Container(
              padding: const EdgeInsets.all(AppTheme.sp12),
              decoration: BoxDecoration(
                color: Colors.grey.withValues(alpha: 0.08),
                borderRadius: BorderRadius.circular(AppTheme.radiusSm),
              ),
              child: const Text(
                '平台仅为撮合平台，不对商品质量承担担保责任。'
                '建议买家在购买前仔细查看商品详情和卖家描述。'
                '如有争议，请联系客服。',
                style: TextStyle(fontSize: 12, color: AppTheme.textSecondary),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TrustSection extends StatelessWidget {
  final IconData icon;
  final Color iconColor;
  final String title;
  final String subtitle;
  final String description;

  const _TrustSection({
    required this.icon,
    required this.iconColor,
    required this.title,
    required this.subtitle,
    required this.description,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          padding: const EdgeInsets.all(10),
          decoration: BoxDecoration(
            color: iconColor.withValues(alpha: 0.1),
            borderRadius: BorderRadius.circular(AppTheme.radiusSm),
          ),
          child: Icon(icon, color: iconColor, size: 22),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 15),
              ),
              const SizedBox(height: 2),
              Text(
                subtitle,
                style: TextStyle(
                  fontSize: 13,
                  color: iconColor,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                description,
                style: const TextStyle(
                  fontSize: 13,
                  color: AppTheme.textSecondary,
                  height: 1.5,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
