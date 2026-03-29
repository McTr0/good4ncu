import 'package:flutter/material.dart';
import '../l10n/app_localizations.dart';
import 'package:go_router/go_router.dart';
import '../models/models.dart';
import '../services/api_service.dart';
import '../services/recommendation_service.dart';
import '../components/recommendation_carousel.dart';
import '../theme/app_theme.dart';
import '../components/price_tag.dart';
import '../components/shimmer_grid.dart';

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  final ApiService _apiService = ApiService();
  final RecommendationService _recommendationService = RecommendationService();
  final TextEditingController _searchController = TextEditingController();

  List<Listing> _listings = [];
  bool _loading = true;
  String? _error;
  String? _selectedCategory;
  int _offset = 0;
  bool _hasMore = true;

  // Recommendation state
  List<Listing> _recommendedListings = [];
  bool _recommendationLoading = true;

  final _categories = [
    'allCategories',
    'electronics',
    'books',
    'digitalAccessories',
    'dailyGoods',
    'clothingShoes',
    'other',
  ];

  String _getCategoryName(BuildContext context, String key) {
    final l = AppLocalizations.of(context)!;
    switch (key) {
      case 'allCategories':
        return l.allCategories;
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

  @override
  void initState() {
    super.initState();
    _loadListings();
    _loadRecommendations();
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _loadListings({bool reset = true}) async {
    if (reset) {
      setState(() {
        _loading = true;
        _error = null;
        _offset = 0;
        _listings = [];
        _hasMore = true;
      });
    }

    try {
      final category = _selectedCategory == 'allCategories' ? null : _selectedCategory;
      final search = _searchController.text.isEmpty ? null : _searchController.text;

      final resp = await _apiService.getListings(
        limit: 20,
        offset: _offset,
        category: category,
        search: search,
      );

      if (mounted) {
        setState(() {
          if (reset) {
            _listings = resp.items;
          } else {
            _listings.addAll(resp.items);
          }
          _hasMore = resp.items.length == 20;
          _loading = false;
        });
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


  Future<void> _loadRecommendations() async {
    setState(() => _recommendationLoading = true);
    try {
      final recommendations = await _recommendationService.getRecommendationFeed(limit: 10);
      if (mounted) {
        setState(() {
          _recommendedListings = recommendations;
          _recommendationLoading = false;
        });
      }
    } catch (e) {
      // Gracefully degrade - hide carousel on error
      if (mounted) {
        setState(() {
          _recommendedListings = [];
          _recommendationLoading = false;
        });
      }
    }
  }

  void _onCategorySelected(String cat) {
    setState(() => _selectedCategory = cat);
    _loadListings();
  }

  void _onSearch(String value) {
    _loadListings();
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
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(AppTheme.sp16),
            child: TextField(
              controller: _searchController,
              decoration: InputDecoration(
                hintText: l.searchHint,
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchController.text.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          _searchController.clear();
                          _onSearch('');
                        },
                      )
                    : null,
              ),
              onSubmitted: _onSearch,
              textInputAction: TextInputAction.search,
            ),
          ),
          SizedBox(
            height: 40,
            child: ListView.separated(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: AppTheme.sp16),
              itemCount: _categories.length,
              separatorBuilder: (context, index) => const SizedBox(width: 8),
              itemBuilder: (context, i) {
                final cat = _categories[i];
                final selected =
                    (_selectedCategory ?? 'allCategories') == cat || (cat == 'allCategories' && _selectedCategory == null);
                return FilterChip(
                  label: Text(_getCategoryName(context, cat)),
                  selected: selected,
                  onSelected: (_) => _onCategorySelected(cat == 'allCategories' ? 'allCategories' : cat),
                  selectedColor: AppTheme.primary.withValues(alpha: 0.15),
                  checkmarkColor: AppTheme.primary,
                  labelStyle: TextStyle(
                    color: selected ? AppTheme.primary : null,
                    fontWeight: selected ? FontWeight.w600 : null,
                  ),
                );
              },
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: _buildContent(),
          ),
        ],
      ),
    );
  }

  Widget _buildContent() {
    final l = AppLocalizations.of(context)!;
    if (_loading && _listings.isEmpty) {
      return const ShimmerGrid();
    }

    if (_error != null && _listings.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.error_outline, size: 48, color: AppTheme.error),
            const SizedBox(height: 16),
            Text(_error!, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => _loadListings(),
              child: Text(l.retry),
            ),
          ],
        ),
      );
    }

    if (_listings.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.inventory_2_outlined, size: 64, color: AppTheme.textSecondary),
            const SizedBox(height: 16),
            Text(
              l.noProducts,
              style: const TextStyle(fontSize: 16, color: AppTheme.textSecondary),
            ),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: () async {
        await _loadListings(reset: true);
        _loadRecommendations();
      },
      child: NotificationListener<ScrollNotification>(
        onNotification: (notification) {
          if (notification is ScrollEndNotification &&
              notification.metrics.extentAfter < 200 &&
              _hasMore &&
              !_loading) {
            setState(() => _offset += 20);
            _loadListings(reset: false);
          }
          return false;
        },
        child: CustomScrollView(
          slivers: [
            // Recommendation carousel
            if (!_recommendationLoading && _recommendedListings.isNotEmpty)
              SliverToBoxAdapter(
                child: Builder(
                  builder: (ctx) => Padding(
                    padding: const EdgeInsets.only(top: AppTheme.sp8, bottom: AppTheme.sp16),
                    child: RecommendationCarousel(
                      listings: _recommendedListings,
                      title: AppLocalizations.of(ctx)?.recommendedForYou ?? '为你推荐',
                    ),
                  ),
                ),
              ),
            // Loading indicator for recommendations
            if (_recommendationLoading)
              const SliverToBoxAdapter(
                child: Padding(
                  padding: EdgeInsets.symmetric(vertical: AppTheme.sp16),
                  child: Center(child: CircularProgressIndicator(strokeWidth: 2)),
                ),
              ),
            // Listings grid
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
                    if (i >= _listings.length) {
                      return const Center(
                        child: Padding(
                          padding: EdgeInsets.all(16),
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                      );
                    }
                    final listing = _listings[i];
                    return ListingCard(
                      listing: listing,
                      onTap: () => context.push('/listing/${listing.id}'),
                    );
                  },
                  childCount: _listings.length + (_hasMore ? 1 : 0),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
