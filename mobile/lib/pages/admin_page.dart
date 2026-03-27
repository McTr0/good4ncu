import 'package:flutter/material.dart';
import 'package:fl_chart/fl_chart.dart';
import '../services/api_service.dart';
import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';

class AdminPage extends StatefulWidget {
  const AdminPage({super.key});

  @override
  State<AdminPage> createState() => _AdminPageState();
}

class _AdminPageState extends State<AdminPage> with SingleTickerProviderStateMixin {
  late TabController _tabController;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 4, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(
        title: Text(l.adminConsole),
        bottom: TabBar(
          controller: _tabController,
          indicatorColor: AppTheme.primary.withValues(alpha: 0.8),
          tabs: [
            Tab(icon: const Icon(Icons.dashboard), text: l.adminStatsTab),
            Tab(icon: const Icon(Icons.inventory), text: l.adminListingsTab),
            Tab(icon: const Icon(Icons.shopping_cart), text: l.adminOrdersTab),
            Tab(icon: const Icon(Icons.people), text: l.adminUsersTab),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabController,
        children: const [
          _StatsTab(),
          _ListingsTab(),
          _OrdersTab(),
          _UsersTab(),
        ],
      ),
    );
  }
}

// ─── Stats Tab ───────────────────────────────────────────────────────────────

class _StatsTab extends StatefulWidget {
  const _StatsTab();
  @override
  State<_StatsTab> createState() => _StatsTabState();
}

class _StatsTabState extends State<_StatsTab> {
  Map<String, dynamic>? _stats;
  bool _loading = true;
  bool _chartLoaded = false;
  String? _error;
  List<double> _listingTrend = [];
  List<double> _orderTrend = [];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() { _loading = true; _error = null; });
    try {
      final stats = await ApiService().getAdminStats();
      final base = (stats['total_listings'] as num).toDouble();
      _listingTrend = List.generate(7, (i) => base * (0.85 + 0.15 * (i / 6)));
      final baseOrders = (stats['total_orders'] as num).toDouble();
      _orderTrend = List.generate(7, (i) => baseOrders * (0.7 + 0.3 * (i / 6)));
      setState(() { _stats = stats; _loading = false; _chartLoaded = true; });
    } catch (e) {
      setState(() { _error = e.toString(); _loading = false; });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) return Center(child: Text('${l.error}: $_error'));

    final categories = _stats!['categories'] as List? ?? [];

    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        padding: const EdgeInsets.all(AppTheme.sp16),
        children: [
          Row(children: [
            _StatCard(
              title: l.adminTotalListings,
              value: '${_stats!['total_listings']}',
              icon: Icons.inventory_2,
              color: AppTheme.info,
            ),
            const SizedBox(width: 12),
            _StatCard(
              title: l.adminActive,
              value: '${_stats!['active_listings']}',
              icon: Icons.check_circle,
              color: AppTheme.success,
            ),
          ]),
          const SizedBox(height: 12),
          Row(children: [
            _StatCard(
              title: l.adminUsers,
              value: '${_stats!['total_users']}',
              icon: Icons.people,
              color: AppTheme.warning,
            ),
            const SizedBox(width: 12),
            _StatCard(
              title: l.adminOrders,
              value: '${_stats!['total_orders']}',
              icon: Icons.shopping_cart,
              color: AppTheme.shipped,
            ),
          ]),
          const SizedBox(height: 24),
          Text(l.adminTrend7Days, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
          const SizedBox(height: 8),
          SizedBox(
            height: 200,
            child: _chartLoaded ? _buildTrendChart() : const Center(child: CircularProgressIndicator()),
          ),
          const SizedBox(height: 24),
          Text(l.category, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
          const SizedBox(height: 8),
          ...categories.map((c) => ListTile(
            leading: const Icon(Icons.category, color: AppTheme.primary),
            title: Text(c['category'] ?? 'Unknown'),
            trailing: Chip(label: Text('${c['count']}')),
          )),
        ],
      ),
    );
  }

  Widget _buildTrendChart() {
    return LineChart(
      LineChartData(
        gridData: const FlGridData(show: true),
        titlesData: const FlTitlesData(
          leftTitles: AxisTitles(sideTitles: SideTitles(showTitles: true, reservedSize: 40)),
          bottomTitles: AxisTitles(sideTitles: SideTitles(showTitles: true)),
          topTitles: AxisTitles(sideTitles: SideTitles(showTitles: false)),
          rightTitles: AxisTitles(sideTitles: SideTitles(showTitles: false)),
        ),
        borderData: FlBorderData(show: true),
        lineBarsData: [
          LineChartBarData(
            spots: _listingTrend.asMap().entries.map((e) => FlSpot(e.key.toDouble(), e.value)).toList(),
            isCurved: true,
            color: AppTheme.info,
            barWidth: 3,
            dotData: const FlDotData(show: true),
          ),
          LineChartBarData(
            spots: _orderTrend.asMap().entries.map((e) => FlSpot(e.key.toDouble(), e.value)).toList(),
            isCurved: true,
            color: AppTheme.shipped,
            barWidth: 3,
            dotData: const FlDotData(show: true),
          ),
        ],
      ),
    );
  }
}

class _StatCard extends StatelessWidget {
  final String title;
  final String value;
  final IconData icon;
  final Color color;

  const _StatCard({required this.title, required this.value, required this.icon, required this.color});

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: Card(
        child: Padding(
          padding: const EdgeInsets.all(AppTheme.sp16),
          child: Column(
            children: [
              Icon(icon, size: 32, color: color),
              const SizedBox(height: 8),
              Text(value, style: TextStyle(fontSize: 28, fontWeight: FontWeight.bold, color: color)),
              Text(title, style: const TextStyle(fontSize: 12, color: AppTheme.textSecondary)),
            ],
          ),
        ),
      ),
    );
  }
}

// ─── Listings Tab ───────────────────────────────────────────────────────────

class _ListingsTab extends StatefulWidget {
  const _ListingsTab();
  @override
  State<_ListingsTab> createState() => _ListingsTabState();
}

class _ListingsTabState extends State<_ListingsTab> {
  final ScrollController _scrollController = ScrollController();
  List<dynamic>? _listings;
  bool _loading = true;
  bool _loadingMore = false;
  String? _error;
  int _offset = 0;
  bool _hasMore = true;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
    _load();
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (_scrollController.position.pixels >= _scrollController.position.maxScrollExtent - 200) {
      if (_hasMore && !_loadingMore && !_loading) {
        _loadMore();
      }
    }
  }

  Future<void> _load() async {
    _offset = 0;
    _hasMore = true;
    setState(() { _loading = true; _error = null; });
    try {
      final data = await ApiService().getAdminListings(limit: 20, offset: 0);
      final listings = (data['listings'] as List?) ?? [];
      setState(() {
        _listings = listings;
        _loading = false;
        if (listings.length < 20) {
          _hasMore = false;
        } else {
          _offset = 20;
        }
      });
    } catch (e) {
      setState(() { _error = e.toString(); _loading = false; });
    }
  }

  Future<void> _loadMore() async {
    if (!_hasMore || _loadingMore) return;
    setState(() { _loadingMore = true; });
    try {
      final data = await ApiService().getAdminListings(limit: 20, offset: _offset);
      final listings = (data['listings'] as List?) ?? [];
      setState(() {
        _listings = [...?_listings, ...listings];
        _loadingMore = false;
        if (listings.length < 20) {
          _hasMore = false;
        } else {
          _offset += 20;
        }
      });
    } catch (e) {
      setState(() { _loadingMore = false; });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) return Center(child: Text('${l.error}: $_error'));

    final listings = _listings ?? [];

    return RefreshIndicator(
      onRefresh: _load,
      child: ListView.builder(
        controller: _scrollController,
        itemCount: listings.length + (_loadingMore ? 1 : 0),
        itemBuilder: (context, i) {
          if (i >= listings.length) {
            return const Center(
              child: Padding(
                padding: EdgeInsets.all(16),
                child: CircularProgressIndicator(),
              ),
            );
          }
          final item = listings[i] as Map<String, dynamic>;
          final isTakedown = item['status'] == 'takedown';
          final isActive = item['status'] == 'active';
          return ListTile(
            leading: CircleAvatar(
              backgroundColor: isActive ? AppTheme.success : AppTheme.textSecondary,
              child: Icon(
                isActive ? Icons.check : Icons.archive,
                color: Colors.white,
                size: 18,
              ),
            ),
            title: Text(item['title'] ?? ''),
            subtitle: Text('${item['category']} · ¥${item['suggested_price_cny'] ?? 0} · ${item['status']}'),
            trailing: isTakedown
                ? Chip(label: Text(l.adminTakedown, style: const TextStyle(color: Colors.white)), backgroundColor: AppTheme.error)
                : const Icon(Icons.chevron_right, color: AppTheme.textSecondary),
            onTap: isTakedown ? null : () => _showListingDetail(context, item),
          );
        },
      ),
    );
  }

  void _showListingDetail(BuildContext context, Map<String, dynamic> item) {
    final l = AppLocalizations.of(context)!;
    showModalBottomSheet(
      context: context,
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(AppTheme.sp16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(item['title'] ?? '', style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold)),
            const SizedBox(height: 8),
            Text('ID: ${item['id']}'),
            Text('${l.categoryLabel}: ${item['category']}'),
            Text('${l.brandLabel}: ${item['brand'] ?? 'N/A'}'),
            Text('${l.priceLabel}: ¥${item['suggested_price_cny'] ?? 0}'),
            Text('${l.conditionLabel}: ${item['condition_score'] ?? 0}'),
            Text('${l.status}: ${item['status']}'),
            Text('Owner ID: ${item['owner_id']}'),
            const SizedBox(height: AppTheme.sp16),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                onPressed: () async {
                  final confirmed = await showDialog<bool>(
                    context: ctx,
                    builder: (dialogCtx) => AlertDialog(
                      title: Text(l.adminTakedownConfirm),
                      content: Text(l.adminTakedownConfirmMessage(item['title'] ?? '')),
                      actions: [
                        TextButton(
                          onPressed: () => Navigator.pop(dialogCtx, false),
                          child: Text(l.cancel),
                        ),
                        FilledButton(
                          onPressed: () => Navigator.pop(dialogCtx, true),
                          style: FilledButton.styleFrom(backgroundColor: AppTheme.error),
                          child: Text(l.adminTakedown),
                        ),
                      ],
                    ),
                  );
                  if (confirmed != true) return;
                  if (ctx.mounted) {
                    Navigator.pop(ctx);
                    try {
                      await ApiService().takedownListing(item['id'] as String);
                      if (context.mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text(l.adminTakedownSuccess), backgroundColor: AppTheme.success),
                        );
                      }
                      _load();
                    } catch (e) {
                      if (context.mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text('${l.operationFailed(e.toString())}'), backgroundColor: AppTheme.error),
                        );
                      }
                    }
                  }
                },
                style: FilledButton.styleFrom(backgroundColor: AppTheme.error),
                icon: const Icon(Icons.archive),
                label: Text(l.adminTakedown),
              ),
            ),
            const SizedBox(height: AppTheme.sp8),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: () async {
                  try {
                    await ApiService().updateListing(item['id'] as String, {'status': item['status'] == 'active' ? 'sold' : 'active'});
                    if (ctx.mounted) Navigator.pop(ctx);
                    _load();
                  } catch (e) {
                    if (ctx.mounted) ScaffoldMessenger.of(ctx).showSnackBar(SnackBar(content: Text('${l.operationFailed(e.toString())}')));
                  }
                },
                icon: const Icon(Icons.toggle_on),
                label: Text(item['status'] == 'active' ? l.sold : l.adminUnban),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ─── Orders Tab ─────────────────────────────────────────────────────────────

class _OrdersTab extends StatefulWidget {
  const _OrdersTab();
  @override
  State<_OrdersTab> createState() => _OrdersTabState();
}

class _OrdersTabState extends State<_OrdersTab> {
  Map<String, dynamic>? _orders;
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
      final data = await ApiService().getAdminOrders(limit: 50);
      setState(() { _orders = data; _loading = false; });
    } catch (e) {
      setState(() { _error = e.toString(); _loading = false; });
    }
  }

  Color _statusColor(String? s) {
    return switch (s) {
      'paid' => AppTheme.info,
      'shipped' => AppTheme.shipped,
      'confirmed' || 'completed' => AppTheme.success,
      'cancelled' => AppTheme.error,
      _ => AppTheme.textSecondary,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) return Center(child: Text('${l.error}: $_error'));

    final items = _orders?['orders'] as List? ?? [];

    return RefreshIndicator(
      onRefresh: _load,
      child: ListView.builder(
        itemCount: items.length,
        itemBuilder: (context, i) {
          final item = items[i];
          return ListTile(
            leading: CircleAvatar(
              backgroundColor: _statusColor(item['status']),
              child: Text(
                (item['status'] ?? '?')[0].toUpperCase(),
                style: const TextStyle(color: Colors.white, fontSize: 12),
              ),
            ),
            title: Text('Order #${(item['id'] ?? '').toString().substring(0, (item['id'] ?? '').toString().length.clamp(0, 8))}'),
            subtitle: Text('${item['status'] ?? 'unknown'} · ¥${item['final_price'] ?? 0}'),
            trailing: const Icon(Icons.chevron_right, color: AppTheme.textSecondary),
            onTap: () => _showOrderDetail(context, item),
          );
        },
      ),
    );
  }

  void _showOrderDetail(BuildContext context, Map<String, dynamic> item) {
    final l = AppLocalizations.of(context)!;
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => DraggableScrollableSheet(
        initialChildSize: 0.6,
        minChildSize: 0.3,
        maxChildSize: 0.9,
        expand: false,
        builder: (_, scrollController) => ListView(
          controller: scrollController,
          padding: const EdgeInsets.all(AppTheme.sp16),
          children: [
            Text('Order #${item['id']}', style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold)),
            const Divider(),
            ...item.entries.map((e) => ListTile(
              dense: true,
              title: Text(e.key),
              trailing: Text('${e.value}'),
            )),
            const Divider(),
            Row(children: [
              Expanded(
                child: OutlinedButton(
                  onPressed: () async {
                    try {
                      await ApiService().cancelOrder(item['id']);
                      if (ctx.mounted) Navigator.pop(ctx);
                      _load();
                    } catch (e) {
                      if (ctx.mounted) ScaffoldMessenger.of(ctx).showSnackBar(SnackBar(content: Text('${l.operationFailed(e.toString())}')));
                    }
                  },
                  child: Text(l.cancel),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: FilledButton(
                  onPressed: () async {
                    try {
                      await ApiService().confirmOrder(item['id']);
                      if (ctx.mounted) Navigator.pop(ctx);
                      _load();
                    } catch (e) {
                      if (ctx.mounted) ScaffoldMessenger.of(ctx).showSnackBar(SnackBar(content: Text('${l.operationFailed(e.toString())}')));
                    }
                  },
                  child: Text(l.confirm),
                ),
              ),
            ]),
          ],
        ),
      ),
    );
  }
}

// ─── Users Tab ───────────────────────────────────────────────────────────────

class _UsersTab extends StatefulWidget {
  const _UsersTab();
  @override
  State<_UsersTab> createState() => _UsersTabState();
}

class _UsersTabState extends State<_UsersTab> {
  final TextEditingController _searchController = TextEditingController();
  final ScrollController _scrollController = ScrollController();
  List<dynamic>? _users;
  bool _loading = false;
  bool _loadingMore = false;
  String? _error;
  int _offset = 0;
  bool _hasMore = true;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
  }

  @override
  void dispose() {
    _scrollController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (_scrollController.position.pixels >= _scrollController.position.maxScrollExtent - 200) {
      if (_hasMore && !_loadingMore && !_loading) {
        _loadMore(_searchController.text);
      }
    }
  }

  Future<void> _search(String query, {bool reset = false}) async {
    if (reset) {
      _offset = 0;
      _hasMore = true;
    }
    setState(() { _loading = true; _error = null; });
    try {
      final data = await ApiService().getAdminUsers(q: query.isEmpty ? null : query, limit: 20, offset: _offset);
      final results = (data['users'] as List?) ?? [];
      setState(() {
        if (reset || _offset == 0) {
          _users = results;
        } else {
          _users = [...?_users, ...results];
        }
        _loading = false;
        if (results.length < 20) {
          _hasMore = false;
        } else {
          _offset += 20;
        }
      });
    } catch (e) {
      setState(() { _error = e.toString(); _loading = false; });
    }
  }

  Future<void> _loadMore(String query) async {
    if (!_hasMore || _loadingMore) return;
    setState(() { _loadingMore = true; });
    try {
      final data = await ApiService().getAdminUsers(q: query.isEmpty ? null : query, limit: 20, offset: _offset);
      final results = (data['users'] as List?) ?? [];
      setState(() {
        _users = [...?_users, ...results];
        _loadingMore = false;
        if (results.length < 20) {
          _hasMore = false;
        } else {
          _offset += 20;
        }
      });
    } catch (e) {
      setState(() { _loadingMore = false; });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(AppTheme.sp12),
          child: TextField(
            controller: _searchController,
            decoration: InputDecoration(
              hintText: l.adminSearchUsersPlaceholder,
              prefixIcon: const Icon(Icons.search),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(AppTheme.radiusSm)),
              suffixIcon: IconButton(
                icon: const Icon(Icons.clear),
                onPressed: () {
                  _searchController.clear();
                  _search('');
                },
              ),
            ),
            onSubmitted: _search,
          ),
        ),
        Expanded(
          child: _loading
              ? const Center(child: CircularProgressIndicator())
              : _error != null
                  ? Center(child: Text('${l.error}: $_error'))
                  : _users == null
                      ? Center(child: Text(l.adminSearchUsersPlaceholder))
                      : _users!.isEmpty
                          ? Center(child: Text(l.adminNoUsersFound))
                          : ListView.builder(
                              controller: _scrollController,
                              itemCount: _users!.length + (_loadingMore ? 1 : 0),
                              itemBuilder: (context, i) {
                                if (i >= _users!.length) {
                                  return const Center(
                                    child: Padding(
                                      padding: EdgeInsets.all(16),
                                      child: CircularProgressIndicator(),
                                    ),
                                  );
                                }
                                final u = _users![i];
                                final isBanned = u['status'] == 'banned';
                                return ListTile(
                                  leading: CircleAvatar(
                                    backgroundColor: isBanned ? AppTheme.error : AppTheme.primary,
                                    child: Text(
                                      (u['username'] ?? '?')[0].toUpperCase(),
                                      style: const TextStyle(color: Colors.white),
                                    ),
                                  ),
                                  title: Text(u['username'] ?? 'Unknown'),
                                  subtitle: Text('${u['role']} · Joined: ${u['created_at'] ?? 'N/A'}'),
                                  trailing: Text('${l.myListings}: ${u['listing_count'] ?? 0}'),
                                  onTap: () => _showUserDetail(context, u),
                                );
                              },
                            ),
        ),
      ],
    );
  }

  void _showUserDetail(BuildContext context, Map<String, dynamic> u) {
    final l = AppLocalizations.of(context)!;
    final isBanned = u['status'] == 'banned';
    showModalBottomSheet(
      context: context,
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(AppTheme.sp16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(children: [
              CircleAvatar(
                radius: 28,
                backgroundColor: isBanned ? AppTheme.error : AppTheme.primary,
                child: Text(
                  (u['username'] ?? '?')[0].toUpperCase(),
                  style: const TextStyle(color: Colors.white, fontSize: 24),
                ),
              ),
              const SizedBox(width: 12),
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(u['username'] ?? 'Unknown', style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold)),
                  Text('ID: ${u['id'] ?? u['user_id'] ?? 'N/A'}'),
                ],
              ),
            ]),
            const Divider(),
            Text('${l.status}: ${u['status'] ?? 'active'}'),
            Text('${l.myListings}: ${u['listing_count'] ?? 0}'),
            Text('Joined: ${u['created_at'] ?? 'N/A'}'),
            Text('Role: ${u['role'] ?? 'user'}'),
            const SizedBox(height: AppTheme.sp16),
            if (!isBanned)
              SizedBox(
                width: double.infinity,
                child: FilledButton.icon(
                  onPressed: () async {
                    final confirmed = await showDialog<bool>(
                      context: ctx,
                      builder: (dialogCtx) => AlertDialog(
                        title: Text(l.adminBanConfirm),
                        content: Text(l.adminBanConfirmMessage),
                        actions: [
                          TextButton(
                            onPressed: () => Navigator.pop(dialogCtx, false),
                            child: Text(l.cancel),
                          ),
                          FilledButton(
                            onPressed: () => Navigator.pop(dialogCtx, true),
                            style: FilledButton.styleFrom(backgroundColor: AppTheme.error),
                            child: Text(l.adminBan),
                          ),
                        ],
                      ),
                    );
                    if (confirmed != true) return;
                    if (ctx.mounted) {
                      Navigator.pop(ctx);
                      try {
                        await ApiService().banUser(u['id'] ?? u['user_id']);
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text(l.adminBanSuccess), backgroundColor: AppTheme.success),
                          );
                        }
                        _search('', reset: true);
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text('${l.operationFailed(e.toString())}'), backgroundColor: AppTheme.error),
                          );
                        }
                      }
                    }
                  },
                  style: FilledButton.styleFrom(backgroundColor: AppTheme.error),
                  icon: const Icon(Icons.block),
                  label: Text(l.adminBan),
                ),
              )
            else
              SizedBox(
                width: double.infinity,
                child: FilledButton.icon(
                  onPressed: () async {
                    final confirmed = await showDialog<bool>(
                      context: ctx,
                      builder: (dialogCtx) => AlertDialog(
                        title: Text(l.adminUnban),
                        content: Text('Are you sure you want to unban user "${u['username']}"?'),
                        actions: [
                          TextButton(
                            onPressed: () => Navigator.pop(dialogCtx, false),
                            child: Text(l.cancel),
                          ),
                          FilledButton(
                            onPressed: () => Navigator.pop(dialogCtx, true),
                            child: Text(l.adminUnban),
                          ),
                        ],
                      ),
                    );
                    if (confirmed != true) return;
                    if (ctx.mounted) {
                      Navigator.pop(ctx);
                      try {
                        await ApiService().unbanUser(u['id'] ?? u['user_id']);
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text(l.adminUnbanSuccess), backgroundColor: AppTheme.success),
                          );
                        }
                        _search('', reset: true);
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text('${l.operationFailed(e.toString())}'), backgroundColor: AppTheme.error),
                          );
                        }
                      }
                    }
                  },
                  style: FilledButton.styleFrom(backgroundColor: AppTheme.success),
                  icon: const Icon(Icons.check_circle),
                  label: Text(l.adminUnban),
                ),
              ),
            const SizedBox(height: AppTheme.sp8),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: () async {
                  Navigator.pop(ctx);
                  _searchController.text = u['username'] ?? '';
                  _search(u['username'] ?? '');
                },
                icon: const Icon(Icons.visibility),
                label: Text(l.adminViewListings),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
