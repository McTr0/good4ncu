import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/api_service.dart';
import '../services/admin_impersonation_service.dart';
import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import 'admin/admin_listings_tab.dart';
import 'admin/admin_orders_tab.dart';
import 'admin/admin_stats_tab.dart';
import 'admin/admin_users_tab.dart';

export 'admin/admin_listings_tab.dart';
export 'admin/admin_orders_tab.dart';
export 'admin/admin_stats_tab.dart';
export 'admin/admin_users_tab.dart';

class AdminPage extends StatefulWidget {
  final ApiService? apiService;
  final AdminImpersonationService? impersonationService;

  const AdminPage({super.key, this.apiService, this.impersonationService});

  @override
  State<AdminPage> createState() => _AdminPageState();
}

class _AdminPageState extends State<AdminPage>
    with SingleTickerProviderStateMixin {
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
    final apiService = widget.apiService ?? context.read<ApiService>();
    final impersonationService =
        widget.impersonationService ??
        context.read<AdminImpersonationService>();
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
        children: [
          AdminStatsTab(apiService: apiService),
          AdminListingsTab(apiService: apiService),
          AdminOrdersTab(apiService: apiService),
          AdminUsersTab(
            apiService: apiService,
            impersonationService: impersonationService,
          ),
        ],
      ),
    );
  }
}
