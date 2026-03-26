import 'package:flutter/material.dart';
import '../services/api_service.dart';
import '../models/models.dart';

class AdminPage extends StatefulWidget {
  const AdminPage({super.key});

  @override
  State<AdminPage> createState() => _AdminPageState();
}

class _AdminPageState extends State<AdminPage> with SingleTickerProviderStateMixin {
  final ApiService _api = ApiService();
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
    return Scaffold(
      appBar: AppBar(
        title: const Text('Admin Console'),
        backgroundColor: Colors.deepPurple,
        foregroundColor: Colors.white,
        bottom: TabBar(
          controller: _tabController,
          labelColor: Colors.white,
          unselectedLabelColor: Colors.white70,
          indicatorColor: Colors.amber,
          tabs: const [
            Tab(icon: Icon(Icons.dashboard), text: 'Stats'),
            Tab(icon: Icon(Icons.inventory), text: 'Listings'),
            Tab(icon: Icon(Icons.shopping_cart), text: 'Orders'),
            Tab(icon: Icon(Icons.people), text: 'Users'),
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
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() { _loading = true; _error = null; });
    try {
      final stats = await ApiService().getAdminStats();
      setState(() { _stats = stats; _loading = false; });
    } catch (e) {
      setState(() { _error = e.toString(); _loading = false; });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) return Center(child: Text('Error: $_error'));

    final categories = _stats!['categories'] as List? ?? [];

    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Row(children: [
            _StatCard(
              title: 'Total Listings',
              value: '${_stats!['total_listings']}',
              icon: Icons.inventory_2,
              color: Colors.blue,
            ),
            const SizedBox(width: 12),
            _StatCard(
              title: 'Active',
              value: '${_stats!['active_listings']}',
              icon: Icons.check_circle,
              color: Colors.green,
            ),
          ]),
          const SizedBox(height: 12),
          Row(children: [
            _StatCard(
              title: 'Users',
              value: '${_stats!['total_users']}',
              icon: Icons.people,
              color: Colors.orange,
            ),
            const SizedBox(width: 12),
            _StatCard(
              title: 'Orders',
              value: '${_stats!['total_orders']}',
              icon: Icons.shopping_cart,
              color: Colors.purple,
            ),
          ]),
          const SizedBox(height: 24),
          const Text('Categories', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
          const SizedBox(height: 8),
          ...categories.map((c) => ListTile(
            leading: const Icon(Icons.category),
            title: Text(c['category'] ?? 'Unknown'),
            trailing: Chip(label: Text('${c['count']}')),
          )),
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
          padding: const EdgeInsets.all(16),
          child: Column(
            children: [
              Icon(icon, size: 32, color: color),
              const SizedBox(height: 8),
              Text(value, style: TextStyle(fontSize: 28, fontWeight: FontWeight.bold, color: color)),
              Text(title, style: const TextStyle(fontSize: 12, color: Colors.grey)),
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
  Map<String, dynamic>? _data;
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
        _data = data;
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
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) return Center(child: Text('Error: $_error'));

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
          return ListTile(
            leading: CircleAvatar(
              backgroundColor: item['status'] == 'active' ? Colors.green : Colors.grey,
              child: Icon(
                item['status'] == 'active' ? Icons.check : Icons.archive,
                color: Colors.white,
                size: 18,
              ),
            ),
            title: Text(item['title'] ?? ''),
            subtitle: Text('${item['category']} · ¥${item['suggested_price_cny'] ?? 0} · ${item['status']}'),
            trailing: isTakedown
                ? const Chip(label: Text('已下架'), backgroundColor: Colors.red)
                : const Icon(Icons.chevron_right),
            onTap: isTakedown ? null : () => _showListingDetail(context, item),
          );
        },
      ),
    );
  }

  void _showListingDetail(BuildContext context, Map<String, dynamic> item) {
    showModalBottomSheet(
      context: context,
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(item['title'] ?? '', style: const TextStyle(fontSize: 20, fontWeight: FontWeight.bold)),
            const SizedBox(height: 8),
            Text('ID: ${item['id']}'),
            Text('Category: ${item['category']}'),
            Text('Brand: ${item['brand'] ?? 'N/A'}'),
            Text('Price: ¥${item['suggested_price_cny'] ?? 0}'),
            Text('Condition: ${item['condition_score'] ?? 0}'),
            Text('Status: ${item['status']}'),
            Text('Owner ID: ${item['owner_id']}'),
            const SizedBox(height: 16),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                onPressed: () async {
                  final confirmed = await showDialog<bool>(
                    context: ctx,
                    builder: (dialogCtx) => AlertDialog(
                      title: const Text('确认下架'),
                      content: Text('确定要强制下架 "${item['title']}" 吗？'),
                      actions: [
                        TextButton(
                          onPressed: () => Navigator.pop(dialogCtx, false),
                          child: const Text('取消'),
                        ),
                        FilledButton(
                          onPressed: () => Navigator.pop(dialogCtx, true),
                          style: FilledButton.styleFrom(backgroundColor: Colors.red),
                          child: const Text('确认下架'),
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
                          const SnackBar(content: Text('商品已强制下架'), backgroundColor: Colors.green),
                        );
                      }
                      _load();
                    } catch (e) {
                      if (context.mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text('Takedown failed: $e'), backgroundColor: Colors.red),
                        );
                      }
                    }
                  }
                },
                style: FilledButton.styleFrom(backgroundColor: Colors.red),
                icon: const Icon(Icons.archive),
                label: const Text('强制下架 (Takedown)'),
              ),
            ),
            const SizedBox(height: 8),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: () async {
                  try {
                    await ApiService().updateListing(item['id'] as String, {'status': item['status'] == 'active' ? 'sold' : 'active'});
                    if (ctx.mounted) Navigator.pop(ctx);
                    _load();
                  } catch (e) {
                    if (ctx.mounted) ScaffoldMessenger.of(ctx).showSnackBar(SnackBar(content: Text('Update failed: $e')));
                  }
                },
                icon: const Icon(Icons.toggle_on),
                label: Text(item['status'] == 'active' ? 'Mark Sold' : 'Reactivate'),
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
      'paid' => Colors.blue,
      'shipped' => Colors.orange,
      'confirmed' || 'completed' => Colors.green,
      'cancelled' => Colors.red,
      _ => Colors.grey,
    };
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) return Center(child: Text('Error: $_error'));

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
            title: Text('Order #${(item['id'] ?? '').toString().substring(0, 8)}'),
            subtitle: Text('${item['status'] ?? 'unknown'} · ¥${item['final_price'] ?? 0}'),
            trailing: Icon(Icons.chevron_right),
            onTap: () => _showOrderDetail(context, item),
          );
        },
      ),
    );
  }

  void _showOrderDetail(BuildContext context, Map<String, dynamic> item) {
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
          padding: const EdgeInsets.all(16),
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
                      if (ctx.mounted) ScaffoldMessenger.of(ctx).showSnackBar(SnackBar(content: Text('Cancel failed: $e')));
                    }
                  },
                  child: const Text('Cancel'),
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
                      if (ctx.mounted) ScaffoldMessenger.of(ctx).showSnackBar(SnackBar(content: Text('Confirm failed: $e')));
                    }
                  },
                  child: const Text('Confirm'),
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
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: TextField(
            controller: _searchController,
            decoration: InputDecoration(
              hintText: 'Search users by username...',
              prefixIcon: const Icon(Icons.search),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
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
                  ? Center(child: Text('Error: $_error'))
                  : _users == null
                      ? const Center(child: Text('Enter username to search'))
                      : _users!.isEmpty
                          ? const Center(child: Text('No users found'))
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
                                return ListTile(
                                  leading: CircleAvatar(
                                    backgroundColor: u['status'] == 'banned' ? Colors.red : Colors.deepPurple,
                                    child: Text(
                                      (u['username'] ?? '?')[0].toUpperCase(),
                                      style: const TextStyle(color: Colors.white),
                                    ),
                                  ),
                                  title: Text(u['username'] ?? 'Unknown'),
                                  subtitle: Text('${u['role']} · Joined: ${u['created_at'] ?? 'N/A'}'),
                                  trailing: Text('Listings: ${u['listing_count'] ?? 0}'),
                                  onTap: () => _showUserDetail(context, u),
                                );
                              },
                            ),
        ),
      ],
    );
  }

  void _showUserDetail(BuildContext context, Map<String, dynamic> u) {
    final isBanned = u['status'] == 'banned';
    showModalBottomSheet(
      context: context,
      builder: (ctx) => Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(children: [
              CircleAvatar(
                radius: 28,
                backgroundColor: isBanned ? Colors.red : Colors.deepPurple,
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
            Text('Status: ${u['status'] ?? 'active'}'),
            Text('Listing count: ${u['listing_count'] ?? 0}'),
            Text('Joined: ${u['created_at'] ?? 'N/A'}'),
            Text('Role: ${u['role'] ?? 'user'}'),
            const SizedBox(height: 16),
            if (!isBanned)
              SizedBox(
                width: double.infinity,
                child: FilledButton.icon(
                  onPressed: () async {
                    final confirmed = await showDialog<bool>(
                      context: ctx,
                      builder: (dialogCtx) => AlertDialog(
                        title: const Text('Confirm Ban'),
                        content: Text('Are you sure you want to ban user "${u['username']}"?'),
                        actions: [
                          TextButton(
                            onPressed: () => Navigator.pop(dialogCtx, false),
                            child: const Text('Cancel'),
                          ),
                          FilledButton(
                            onPressed: () => Navigator.pop(dialogCtx, true),
                            style: FilledButton.styleFrom(backgroundColor: Colors.red),
                            child: const Text('Ban'),
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
                            const SnackBar(content: Text('用户已封禁'), backgroundColor: Colors.green),
                          );
                        }
                        _search('', reset: true);
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text('Ban failed: $e'), backgroundColor: Colors.red),
                          );
                        }
                      }
                    }
                  },
                  style: FilledButton.styleFrom(backgroundColor: Colors.red),
                  icon: const Icon(Icons.block),
                  label: const Text('封禁用户 (Ban)'),
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
                        title: const Text('Confirm Unban'),
                        content: Text('Are you sure you want to unban user "${u['username']}"?'),
                        actions: [
                          TextButton(
                            onPressed: () => Navigator.pop(dialogCtx, false),
                            child: const Text('Cancel'),
                          ),
                          FilledButton(
                            onPressed: () => Navigator.pop(dialogCtx, true),
                            child: const Text('Unban'),
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
                            const SnackBar(content: Text('用户已解封'), backgroundColor: Colors.green),
                          );
                        }
                        _search('', reset: true);
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text('Unban failed: $e'), backgroundColor: Colors.red),
                          );
                        }
                      }
                    }
                  },
                  style: FilledButton.styleFrom(backgroundColor: Colors.green),
                  icon: const Icon(Icons.check_circle),
                  label: const Text('解封用户 (Unban)'),
                ),
              ),
            const SizedBox(height: 8),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: () async {
                  Navigator.pop(ctx);
                  _searchController.text = u['username'] ?? '';
                  _search(u['username'] ?? '');
                },
                icon: const Icon(Icons.visibility),
                label: const Text('View Listings'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
