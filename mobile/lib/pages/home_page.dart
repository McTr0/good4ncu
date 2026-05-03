import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:go_router/go_router.dart';
import '../l10n/app_localizations.dart';
import '../models/models.dart';
import '../services/recommendation_service.dart';
import '../theme/app_theme.dart';
import '../components/price_tag.dart';

class HomePage extends StatefulWidget {
  final RecommendationService? recommendationService;

  const HomePage({super.key, this.recommendationService});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  late final RecommendationService _recommendationService;

  // Recommendation state
  List<Listing> _recommendedListings = [];
  bool _recommendationLoading = true;
  bool _feedHasMore = true;
  bool _feedLoading = false;

  @override
  void initState() {
    super.initState();
    _recommendationService =
        widget.recommendationService ?? context.read<RecommendationService>();
    _loadRecommendations();
  }

  Future<void> _loadRecommendations({bool reset = true}) async {
    if (reset) {
      setState(() => _recommendationLoading = true);
    }
    try {
      final recommendations = await _recommendationService
          .getRecommendationFeed(
            limit: 20,
            offset: reset ? 0 : _recommendedListings.length,
          );
      if (mounted) {
        setState(() {
          if (reset) {
            _recommendedListings = recommendations;
          } else {
            _recommendedListings.addAll(recommendations);
          }
          _feedHasMore = recommendations.length == 20;
          _recommendationLoading = false;
          _feedLoading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _recommendedListings = [];
          _recommendationLoading = false;
          _feedLoading = false;
        });
      }
    }
  }

  @override
  void dispose() {
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(
        title: Text(
          l.appTitle,
          style: const TextStyle(fontWeight: FontWeight.bold),
        ),
      ),
      body: _buildContent(l),
    );
  }

  Widget _buildContent(AppLocalizations l) {
    if (_recommendationLoading && _recommendedListings.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_recommendedListings.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(
              Icons.recommend_outlined,
              size: 64,
              color: AppTheme.textSecondary,
            ),
            const SizedBox(height: 16),
            Text(
              l.noProducts,
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
      onRefresh: () async {
        await _loadRecommendations(reset: true);
      },
      child: NotificationListener<ScrollNotification>(
        onNotification: (notification) {
          if (notification is ScrollEndNotification &&
              notification.metrics.extentAfter < 200 &&
              _feedHasMore &&
              !_feedLoading) {
            setState(() => _feedLoading = true);
            _loadRecommendations(reset: false);
          }
          return false;
        },
        child: CustomScrollView(
          slivers: [
            SliverPadding(
              padding: const EdgeInsets.all(AppTheme.sp16),
              sliver: SliverGrid(
                gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                  crossAxisCount: 2,
                  childAspectRatio: 0.72,
                  crossAxisSpacing: 12,
                  mainAxisSpacing: 12,
                ),
                delegate: SliverChildBuilderDelegate(
                  (context, i) {
                    if (i >= _recommendedListings.length) {
                      return const Center(
                        child: Padding(
                          padding: EdgeInsets.all(16),
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                      );
                    }
                    final listing = _recommendedListings[i];
                    return ListingCard(
                      listing: listing,
                      onTap: () => context.push('/listing/${listing.id}'),
                    );
                  },
                  childCount:
                      _recommendedListings.length + (_feedHasMore ? 1 : 0),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
