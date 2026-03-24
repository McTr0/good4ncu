import 'package:flutter/material.dart';
import '../models/models.dart';
import '../theme/app_theme.dart';

class PriceTag extends StatelessWidget {
  final double priceCny;
  final double fontSize;
  final FontWeight fontWeight;

  const PriceTag({
    super.key,
    required this.priceCny,
    this.fontSize = 18,
    this.fontWeight = FontWeight.bold,
  });

  @override
  Widget build(BuildContext context) {
    return Text(
      '¥${priceCny.toStringAsFixed(2)}',
      style: TextStyle(
        fontSize: fontSize,
        fontWeight: fontWeight,
        color: AppTheme.primary,
      ),
    );
  }
}

class ConditionBadge extends StatelessWidget {
  final int score;
  final String label;
  final Color color;

  const ConditionBadge({
    super.key,
    required this.score,
    required this.label,
    required this.color,
  });

  factory ConditionBadge.fromScore(int score) {
    Color color;
    String label;
    if (score >= 9) {
      color = AppTheme.success;
      label = '几乎全新';
    } else if (score >= 7) {
      color = AppTheme.info;
      label = '较好';
    } else if (score >= 5) {
      color = AppTheme.warning;
      label = '一般';
    } else {
      color = AppTheme.error;
      label = '较差';
    }
    return ConditionBadge(score: score, label: label, color: color);
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.12),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 12,
          fontWeight: FontWeight.w600,
          color: color,
        ),
      ),
    );
  }
}

class ListingCard extends StatelessWidget {
  final Listing listing;
  final VoidCallback onTap;

  const ListingCard({
    super.key,
    required this.listing,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        decoration: BoxDecoration(
          color: Theme.of(context).cardTheme.color,
          borderRadius: BorderRadius.circular(AppTheme.radiusMd),
          border: Border.all(
            color: Theme.of(context).dividerColor,
            width: 1,
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Thumbnail placeholder
            Expanded(
              flex: 3,
              child: Container(
                width: double.infinity,
                color: AppTheme.primary.withOpacity(0.08),
                child: Center(
                  child: Icon(
                    Icons.inventory_2_outlined,
                    size: 48,
                    color: AppTheme.primary.withOpacity(0.4),
                  ),
                ),
              ),
            ),
            // Info
            Expanded(
              flex: 2,
              child: Padding(
                padding: const EdgeInsets.all(AppTheme.sp8),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      listing.title,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        fontWeight: FontWeight.w600,
                        fontSize: 13,
                        height: 1.3,
                      ),
                    ),
                    const Spacer(),
                    Row(
                      children: [
                        PriceTag(
                          priceCny: listing.suggestedPriceCny,
                          fontSize: 15,
                        ),
                        const Spacer(),
                        ConditionBadge.fromScore(listing.conditionScore),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
