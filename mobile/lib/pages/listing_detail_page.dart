import 'package:flutter/material.dart';
import '../l10n/app_localizations.dart';
import 'package:go_router/go_router.dart';
import '../models/models.dart';
import '../services/api_service.dart';
import '../services/base_service.dart';
import '../services/recommendation_service.dart';
import '../services/order_service.dart';
import '../theme/app_theme.dart';
import '../components/price_tag.dart';
import '../components/recommendation_carousel.dart';

class ListingDetailPage extends StatefulWidget {
  final String listingId;

  const ListingDetailPage({super.key, required this.listingId});

  @override
  State<ListingDetailPage> createState() => _ListingDetailPageState();
}

class _ListingDetailPageState extends State<ListingDetailPage> {
  final ApiService _apiService = ApiService();
  final RecommendationService _recommendationService = RecommendationService();
  final OrderService _orderService = OrderService();
  Listing? _listing;
  bool _loading = true;
  String? _error;
  bool _isOperating = false;

  // Similar listings state
  List<Listing> _similarListings = [];
  bool _similarLoading = true;

  @override
  void initState() {
    super.initState();
    _loadDetail();
  }

  Future<void> _loadDetail() async {
    setState(() => _loading = true);
    try {
      final listing = await _apiService.getListingDetail(widget.listingId);
      if (mounted) {
        setState(() {
          _listing = listing;
          _loading = false;
        });
        _loadSimilarListings();
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _loading = false;
          _error = e.toString();
        });
      }
    }
  }

  Future<void> _loadSimilarListings() async {
    setState(() => _similarLoading = true);
    try {
      final similar = await _recommendationService.getSimilarListings(
        widget.listingId,
      );
      if (mounted) {
        setState(() {
          _similarListings = similar;
          _similarLoading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _similarListings = [];
          _similarLoading = false;
        });
      }
    }
  }

  Future<void> _handleContactSeller(BuildContext context) async {
    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    final router = GoRouter.of(context);
    if (_isOperating) return;
    final listing = _listing;
    if (listing == null || listing.ownerId == null) {
      messenger.showSnackBar(
        SnackBar(
          content: Text(l.cannotContactSeller),
          backgroundColor: AppTheme.error,
        ),
      );
      return;
    }

    setState(() => _isOperating = true);

    try {
      final profile = await _apiService.getUserProfile();
      if (!mounted) return;
      final currentUserId = profile['user_id']?.toString();
      if (currentUserId == listing.ownerId) {
        messenger.showSnackBar(SnackBar(content: Text(l.chatWithSelf)));
        setState(() => _isOperating = false);
        return;
      }

      final connections = await _apiService.getConnections();
      if (!mounted) return;
      final existing = connections
          .where((c) => c.otherUserId == listing.ownerId)
          .toList();

      if (existing.isNotEmpty) {
        final conv = existing.first;
        router.push(
          '/chat/${conv.id}',
          extra: {
            'conversationId': conv.id,
            'otherUserId': conv.otherUserId,
            'otherUsername': conv.otherUsername,
          },
        );
      } else {
        await _apiService.requestConnection(
          listing.ownerId!,
          listingId: listing.id,
        );
        if (!mounted) return;
        messenger.showSnackBar(
          SnackBar(
            content: Text(l.connectionRequestSent),
            duration: const Duration(seconds: 3),
          ),
        );
        router.push('/conversations');
      }
    } catch (e) {
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(
          content: Text(l.operationFailed(e.toString())),
          backgroundColor: AppTheme.error,
        ),
      );
    } finally {
      if (mounted) setState(() => _isOperating = false);
    }
  }

  Future<void> _handleBuyNow() async {
    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    final router = GoRouter.of(context);
    if (_isOperating) return;
    final listing = _listing;
    if (listing == null) return;

    setState(() => _isOperating = true);
    try {
      Map<String, dynamic> userProfile;
      try {
        userProfile = await _apiService.getUserProfile();
      } on AuthException {
        if (!mounted) return;
        messenger.showSnackBar(SnackBar(content: Text(l.sessionExpired)));
        return;
      } catch (e) {
        if (!mounted) return;
        messenger.showSnackBar(
          SnackBar(
            content: Text(l.operationFailed(e.toString())),
            backgroundColor: AppTheme.error,
          ),
        );
        return;
      }

      if (userProfile['user_id'] == listing.ownerId) {
        messenger.showSnackBar(SnackBar(content: Text(l.chatWithSelf)));
        return;
      }

      if (!mounted) return;

      final confirm = await showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: Text(l.buyNow),
          content: Text('${l.priceLabel}: ¥${listing.suggestedPriceCny}'),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: Text(l.cancel),
            ),
            ElevatedButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: Text(l.confirm),
            ),
          ],
        ),
      );

      if (confirm != true) return;

      final res = await _orderService.createOrder(
        listingId: listing.id,
        offeredPriceCny: listing.suggestedPriceCny,
      );
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(
          content: Text(l.purchaseSuccess),
          backgroundColor: AppTheme.success,
        ),
      );
      router.push('/orders/${res['id']}');
    } catch (e) {
      if (!mounted) return;
      messenger.showSnackBar(
        SnackBar(
          content: Text(l.operationFailed(e.toString())),
          backgroundColor: AppTheme.error,
        ),
      );
    } finally {
      if (mounted) setState(() => _isOperating = false);
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
            ElevatedButton(onPressed: _loadDetail, child: Text(l.retry)),
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
              color: AppTheme.primary.withValues(alpha: 0.08),
              borderRadius: BorderRadius.circular(AppTheme.radiusMd),
            ),
            child: Center(
              child: Icon(
                Icons.inventory_2_outlined,
                size: 80,
                color: AppTheme.primary.withValues(alpha: 0.3),
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
              Flexible(
                child: PriceTag(
                  priceCny: listing.suggestedPriceCny,
                  fontSize: 28,
                ),
              ),
              const SizedBox(width: 12),
              conditionBadgeFromScore(listing.conditionScore),
            ],
          ),
          const SizedBox(height: AppTheme.sp20),
          const Divider(),
          const SizedBox(height: AppTheme.sp16),
          _DetailRow(
            label: l.categoryLabel,
            value: _getCategoryDisplayName(context, listing.category),
          ),
          _DetailRow(label: l.brandLabel, value: listing.brand),
          _DetailRow(
            label: l.conditionLabel,
            value: '${listing.conditionScore}/10',
          ),
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
                  .map(
                    (d) => Chip(
                      label: Text(d, style: const TextStyle(fontSize: 13)),
                      backgroundColor: AppTheme.error.withValues(alpha: 0.1),
                      labelStyle: const TextStyle(color: AppTheme.error),
                      side: BorderSide.none,
                    ),
                  )
                  .toList(),
            ),
          ],
          if (listing.description != null &&
              listing.description!.isNotEmpty) ...[
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
                  backgroundColor: AppTheme.primary.withValues(alpha: 0.15),
                  child: const Icon(Icons.person, color: AppTheme.primary),
                ),
                const SizedBox(width: 12),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      l.owner,
                      style: const TextStyle(
                        fontSize: 12,
                        color: AppTheme.textSecondary,
                      ),
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
          const SizedBox(height: AppTheme.sp20),
          Container(
            padding: const EdgeInsets.all(AppTheme.sp12),
            decoration: BoxDecoration(
              color: Colors.orange.withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(AppTheme.radiusSm),
              border: Border.all(color: Colors.orange.withValues(alpha: 0.3)),
            ),
            child: Row(
              children: [
                const Icon(Icons.info_outline, color: Colors.orange, size: 20),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    l.infoDisclaimer,
                    style: const TextStyle(fontSize: 13, color: Colors.orange),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: AppTheme.sp24),
          if (_similarLoading)
            const Center(child: CircularProgressIndicator(strokeWidth: 2))
          else if (_similarListings.isNotEmpty)
            RecommendationCarousel(
              listings: _similarListings,
              title: l.similarRecommendations,
            ),
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
    final isSold = _listing?.status == 'sold';

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
              flex: 1,
              child: OutlinedButton.icon(
                onPressed: _isOperating
                    ? null
                    : () => _handleContactSeller(context),
                icon: const Icon(Icons.chat_bubble_outline),
                label: Text(l.contactSeller),
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 12),
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              flex: 1,
              child: ElevatedButton.icon(
                onPressed: (isSold || _isOperating) ? null : _handleBuyNow,
                icon: Icon(isSold ? Icons.done : Icons.shopping_cart_checkout),
                label: Text(isSold ? l.sold : l.buyNow),
                style: ElevatedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 12),
                  backgroundColor: isSold ? Colors.grey : AppTheme.primary,
                  foregroundColor: Colors.white,
                ),
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
              style: const TextStyle(
                color: AppTheme.textSecondary,
                fontSize: 14,
              ),
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
