import 'package:flutter/material.dart';
import '../l10n/app_localizations.dart';
import 'package:go_router/go_router.dart';
import '../models/models.dart';
import '../services/api_service.dart';
import '../theme/app_theme.dart';
import '../components/price_tag.dart';

class ListingDetailPage extends StatefulWidget {
  final String listingId;

  const ListingDetailPage({super.key, required this.listingId});

  @override
  State<ListingDetailPage> createState() => _ListingDetailPageState();
}

class _ListingDetailPageState extends State<ListingDetailPage> {
  final ApiService _apiService = ApiService();
  Listing? _listing;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadDetail();
  }

  Future<void> _loadDetail() async {
    setState(() => _loading = true);
    try {
      final listing = await _apiService.getListingDetail(widget.listingId);
      if (mounted) setState(() { _listing = listing; _loading = false; });
    } catch (e) {
      if (mounted) setState(() { _loading = false; _error = e.toString(); });
    }
  }

  Future<void> _handleContactSeller(BuildContext context) async {
    final listing = _listing;
    if (listing == null || listing.ownerId == null) return;

    // Can't chat with yourself
    try {
      final profile = await _apiService.getUserProfile();
      if (!mounted) return;
      final currentUserId = profile['user_id']?.toString();
      if (currentUserId == listing.ownerId) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('不能和自己聊天')),
        );
        return;
      }
    } catch (_) {}

    try {
      // Check existing connections
      final connections = await _apiService.getConnections();
      if (!mounted) return;
      final existing = connections.where((c) => c.otherUserId == listing.ownerId).toList();

      if (existing.isNotEmpty) {
        // Already have a connection — go to chat
        final conv = existing.first;
        context.push('/chat/${conv.id}', extra: {
          'conversationId': conv.id,
          'otherUserId': conv.otherUserId,
          'otherUsername': conv.otherUsername,
        });
      } else {
        // Send connection request
        await _apiService.requestConnection(listing.ownerId!, listingId: listing.id);
        if (!mounted) return;
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('已发送连接请求，等待对方接受'), duration: Duration(seconds: 3)),
        );
        // Navigate to conversation list so user can see the pending state
        context.push('/conversations');
      }
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('连接失败: $e'), duration: const Duration(seconds: 3)),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(
        title: Text(_listing?.title ?? l.listingDetail),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.pop(),
        ),
      ),
      body: _buildBody(),
      bottomNavigationBar: _listing != null ? _buildBottomBar() : null,
    );
  }

  Widget _buildBody() {
    final l = AppLocalizations.of(context)!;
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
              onPressed: _loadDetail,
              child: Text(l.retry),
            ),
          ],
        ),
      );
    }

    final listing = _listing!;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(AppTheme.sp16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            height: 240,
            width: double.infinity,
            decoration: BoxDecoration(
              color: AppTheme.primary.withOpacity(0.08),
              borderRadius: BorderRadius.circular(AppTheme.radiusMd),
            ),
            child: Center(
              child: Icon(
                Icons.inventory_2_outlined,
                size: 80,
                color: AppTheme.primary.withOpacity(0.3),
              ),
            ),
          ),
          const SizedBox(height: AppTheme.sp20),
          Text(
            listing.title,
            style: const TextStyle(
              fontSize: 22,
              fontWeight: FontWeight.bold,
              height: 1.3,
            ),
          ),
          const SizedBox(height: AppTheme.sp12),
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              PriceTag(
                priceCny: listing.suggestedPriceCny,
                fontSize: 28,
              ),
              const SizedBox(width: 12),
              ConditionBadge.fromScore(listing.conditionScore),
            ],
          ),
          const SizedBox(height: AppTheme.sp20),
          const Divider(),
          const SizedBox(height: AppTheme.sp16),
          _DetailRow(label: l.categoryLabel, value: _getCategoryDisplayName(context, listing.category)),
          _DetailRow(label: l.brandLabel, value: listing.brand),
          _DetailRow(label: l.conditionLabel, value: '${listing.conditionScore}/10'),
          if (listing.defects != null && listing.defects!.isNotEmpty) ...[
            const SizedBox(height: AppTheme.sp16),
            Text(
              l.defectsLabel,
              style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15),
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: listing.defects!
                  .map((d) => Chip(
                        label: Text(d, style: const TextStyle(fontSize: 13)),
                        backgroundColor: AppTheme.error.withOpacity(0.1),
                        labelStyle: const TextStyle(color: AppTheme.error),
                        side: BorderSide.none,
                      ))
                  .toList(),
            ),
          ],
          if (listing.description != null && listing.description!.isNotEmpty) ...[
            const SizedBox(height: AppTheme.sp16),
            Text(
              l.descriptionLabel,
              style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 15),
            ),
            const SizedBox(height: 8),
            Text(
              listing.description!,
              style: const TextStyle(
                fontSize: 14,
                color: AppTheme.textSecondary,
                height: 1.6,
              ),
            ),
          ],
          if (listing.ownerUsername != null) ...[
            const SizedBox(height: AppTheme.sp20),
            const Divider(),
            const SizedBox(height: AppTheme.sp16),
            Row(
              children: [
                CircleAvatar(
                  backgroundColor: AppTheme.primary.withOpacity(0.15),
                  child: const Icon(Icons.person, color: AppTheme.primary),
                ),
                const SizedBox(width: 12),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      l.owner,
                      style: const TextStyle(fontSize: 12, color: AppTheme.textSecondary),
                    ),
                    Text(
                      listing.ownerUsername!,
                      style: const TextStyle(fontWeight: FontWeight.w600),
                    ),
                  ],
                ),
              ],
            ),
          ],
          const SizedBox(height: 100),
        ],
      ),
    );
  }

  String _getCategoryDisplayName(BuildContext context, String key) {
    final l = AppLocalizations.of(context)!;
    switch (key) {
      case 'electronics':
        return l.electronics;
      case 'books':
        return l.books;
      case 'digitalAccessories':
        return l.digitalAccessories;
      case 'dailyGoods':
        return l.dailyGoods;
      case 'clothingShoes':
        return l.clothingShoes;
      case 'other':
        return l.other;
      default:
        return key;
    }
  }

  Widget _buildBottomBar() {
    final l = AppLocalizations.of(context)!;
    return Container(
      padding: const EdgeInsets.all(AppTheme.sp16),
      decoration: BoxDecoration(
        color: Theme.of(context).cardTheme.color,
        border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: SafeArea(
        child: Row(
          children: [
            Expanded(
              child: OutlinedButton.icon(
                onPressed: () => _handleContactSeller(context),
                icon: const Icon(Icons.chat_bubble_outline),
                label: Text(l.contactSeller),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: ElevatedButton(
                onPressed: () {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Purchase coming soon...')),
                  );
                },
                child: Text(l.buyNow),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _DetailRow extends StatelessWidget {
  final String label;
  final String value;

  const _DetailRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 80,
            child: Text(
              label,
              style: const TextStyle(color: AppTheme.textSecondary, fontSize: 14),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500),
            ),
          ),
        ],
      ),
    );
  }
}
