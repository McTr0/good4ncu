import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

/// Escrow protection badge — shows platform holds funds during transaction.
class EscrowBadge extends StatelessWidget {
  /// Show compact (icon + amount) or full (icon + title + amount + explanation).
  final bool compact;
  final double? amountCny;

  const EscrowBadge({
    super.key,
    this.compact = false,
    this.amountCny,
  });

  @override
  Widget build(BuildContext context) {
    if (compact) {
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.lock, size: 14, color: AppTheme.success),
          if (amountCny != null) ...[
            const SizedBox(width: 4),
            Text(
              '¥${amountCny!.toStringAsFixed(2)}',
              style: const TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.w600,
                color: AppTheme.success,
              ),
            ),
          ],
        ],
      );
    }

    return Container(
      padding: const EdgeInsets.all(AppTheme.sp12),
      decoration: BoxDecoration(
        color: AppTheme.success.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(AppTheme.radiusSm),
        border: Border.all(color: AppTheme.success.withValues(alpha: 0.3)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.lock, size: 18, color: AppTheme.success),
              const SizedBox(width: 8),
              const Text(
                '平台托管',
                style: TextStyle(
                  fontWeight: FontWeight.bold,
                  fontSize: 14,
                  color: AppTheme.success,
                ),
              ),
            ],
          ),
          if (amountCny != null) ...[
            const SizedBox(height: 8),
            Text(
              '¥${amountCny!.toStringAsFixed(2)}',
              style: const TextStyle(
                fontSize: 20,
                fontWeight: FontWeight.bold,
                color: AppTheme.success,
              ),
            ),
          ],
          const SizedBox(height: 4),
          Text(
            '款项由平台临时托管，确认收货后自动放款给卖家',
            style: TextStyle(
              fontSize: 12,
              color: AppTheme.textSecondary.withValues(alpha: 0.9),
            ),
          ),
        ],
      ),
    );
  }
}
