import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import '../l10n/app_localizations.dart';
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
    final formatter = NumberFormat.currency(symbol: '\u00A5', decimalDigits: 2);
    return Text(
      formatter.format(priceCny),
      overflow: TextOverflow.ellipsis,
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
  final Color color;

  const ConditionBadge({
    super.key,
    required this.score,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    String label;
    if (score >= 9) {
      label = l.conditionLikeNew;
    } else if (score >= 7) {
      label = l.conditionGood;
    } else if (score >= 5) {
      label = l.conditionFair;
    } else {
      label = l.conditionPoor;
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
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

ConditionBadge conditionBadgeFromScore(int score) {
  Color color;
  if (score >= 9) {
    color = AppTheme.success;
  } else if (score >= 7) {
    color = AppTheme.info;
  } else if (score >= 5) {
    color = AppTheme.warning;
  } else {
    color = AppTheme.error;
  }
  return ConditionBadge(score: score, color: color);
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
                color: AppTheme.primary.withValues(alpha: 0.08),
                child: Center(
                  child: Icon(
                    Icons.inventory_2_outlined,
                    size: 48,
                    color: AppTheme.primary.withValues(alpha: 0.4),
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
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Flexible(
                          child: PriceTag(
                            priceCny: listing.suggestedPriceCny,
                            fontSize: 15,
                          ),
                        ),
                        const SizedBox(width: 4),
                        conditionBadgeFromScore(listing.conditionScore),
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
