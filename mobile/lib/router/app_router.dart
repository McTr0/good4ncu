import 'package:flutter/material.dart';
import '../l10n/app_localizations.dart';
import 'package:go_router/go_router.dart';
import '../pages/home_page.dart';
import '../pages/listing_detail_page.dart';
import '../pages/create_listing_page.dart';
import '../pages/my_listings_page.dart';
import '../pages/profile_page.dart';
import '../pages/chat_page.dart';
import '../pages/conversation_list_page.dart';
import '../pages/user_chat_page.dart';
import '../pages/login_page.dart';
import '../pages/my_orders_page.dart';
import '../pages/order_detail_page.dart';
import '../pages/admin_page.dart';
import '../pages/settings_page.dart';
import '../pages/watchlist_page.dart';
import '../pages/notifications_page.dart';
import '../services/base_service.dart';
import '../services/chat_service.dart';
import '../services/user_service.dart';
import '../services/admin_role_cache.dart';
import '../pages/trust_page.dart';
import '../services/sse_service.dart';
import '../services/token_storage.dart';
import '../services/ws_service.dart';
import '../components/floating_agent_bubble.dart';
import '../providers/agent_chat_notifier.dart';
import 'package:provider/provider.dart';

final GlobalKey<NavigatorState> _rootNavigatorKey = BaseService.navigatorKey;

Future<bool> getLoginStatus() async {
  final token = await TokenStorage.instance.getAccessToken();
  return token != null && token.isNotEmpty;
}

Future<bool> _isAdmin(UserService userService) async {
  try {
    final cached = await AdminRoleCache.instance.getCachedForCurrentToken();
    if (cached != null) {
      return cached;
    }

    final profile = await userService.getUserProfile();
    final isAdmin = profile['role'] == 'admin';
    await AdminRoleCache.instance.saveForCurrentToken(isAdmin);
    return isAdmin;
  } catch (_) {
    AdminRoleCache.instance.invalidate();
    return false;
  }
}

final GoRouter appRouter = GoRouter(
  navigatorKey: _rootNavigatorKey,
  initialLocation: '/',
  redirect: (context, state) async {
    try {
      final userService = context.read<UserService>();
      final loggedIn = await getLoginStatus();
      final onAuthRoute = state.matchedLocation == '/login';
      if (!loggedIn && !onAuthRoute) {
        return '/login';
      }
      if (loggedIn && onAuthRoute) {
        WsService.instance.connect();
        return '/';
      }
      if (loggedIn) {
        WsService.instance.connect();
      }
      if (state.matchedLocation == '/admin') {
        final admin = await _isAdmin(userService);
        if (!admin) return '/';
      }
    } catch (e) {
      if (state.matchedLocation != '/login') {
        return '/login';
      }
    }
    return null;
  },
  routes: [
    GoRoute(path: '/login', builder: (context, state) => const LoginPage()),
    GoRoute(path: '/trust', builder: (context, state) => const TrustPage()),
    GoRoute(
      path: '/settings',
      builder: (context, state) => const SettingsPage(),
    ),
    GoRoute(path: '/admin', builder: (context, state) => const AdminPage()),
    GoRoute(
      path: '/watchlist',
      builder: (context, state) => const WatchlistPage(),
    ),
    GoRoute(
      path: '/notifications',
      builder: (context, state) => const NotificationsPage(),
    ),

    // Detail routes (Hide bottom bar)
    GoRoute(
      path: '/listing/:id',
      builder: (context, state) {
        final id = state.pathParameters['id']!;
        return ListingDetailPage(listingId: id);
      },
    ),
    GoRoute(
      path: '/orders/:id',
      builder: (context, state) {
        final id = state.pathParameters['id']!;
        return OrderDetailPage(orderId: id);
      },
    ),
    GoRoute(
      path: '/chat/:conversationId',
      builder: (context, state) {
        final id = state.pathParameters['conversationId']!;
        final extra = state.extra as Map<String, dynamic>?;
        final otherUserId = extra?['otherUserId'] as String? ?? '';
        final otherUsername = extra?['otherUsername'] as String? ?? '';
        return UserChatPage(
          conversationId: id,
          otherUserId: otherUserId,
          otherUsername: otherUsername,
        );
      },
    ),

    // Tab routes (Show bottom bar)
    ShellRoute(
      builder: (context, state, child) => _ShellScaffold(child: child),
      routes: [
        GoRoute(
          path: '/',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: HomePage()),
        ),
        GoRoute(
          path: '/conversations',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: ConversationListPage()),
        ),
        GoRoute(
          path: '/create',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: CreateListingPage()),
        ),
        GoRoute(
          path: '/profile',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: ProfilePage()),
        ),
        GoRoute(
          path: '/my-listings',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: MyListingsPage()),
        ),
        GoRoute(
          path: '/orders',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: MyOrdersPage()),
        ),
        GoRoute(
          path: '/chat',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: ChatPage()),
        ),
      ],
    ),
  ],
);

class _ShellScaffold extends StatefulWidget {
  final Widget child;
  const _ShellScaffold({required this.child});

  @override
  State<_ShellScaffold> createState() => _ShellScaffoldState();
}

class _ShellScaffoldState extends State<_ShellScaffold> {
  int _currentIndex = 0;

  static const _routes = ['/', '/conversations', '/create', '/profile'];

  bool _isLoggedInStatus = false;
  late final AgentChatNotifier _agentChatNotifier;

  @override
  void initState() {
    super.initState();
    _agentChatNotifier = AgentChatNotifier(
      sseService: context.read<SseService>(),
      chatService: context.read<ChatService>(),
      userService: context.read<UserService>(),
    );
    _checkLoginStatus();
  }

  @override
  void dispose() {
    _agentChatNotifier.dispose();
    super.dispose();
  }

  Future<void> _checkLoginStatus() async {
    final loggedIn = await getLoginStatus();
    if (mounted) {
      setState(() => _isLoggedInStatus = loggedIn);
    }
  }

  int _tabIndexForLocation(String location) {
    switch (location) {
      case '/':
        return 0;
      case '/conversations':
      case '/chat':
        return 1;
      case '/create':
        return 2;
      case '/profile':
      case '/my-listings':
      case '/orders':
        return 3;
      default:
        return _currentIndex;
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    if (l == null) {
      return const SizedBox.shrink();
    }
    final location = GoRouterState.of(context).matchedLocation;
    final nextIndex = _tabIndexForLocation(location);
    if (_currentIndex != nextIndex) {
      _currentIndex = nextIndex;
    }

    return ChangeNotifierProvider<AgentChatNotifier>.value(
      value: _agentChatNotifier,
      child: Scaffold(
        body: Stack(
          fit: StackFit.expand,
          children: [
            widget.child,
            FloatingAgentBubble(isLoggedIn: _isLoggedInStatus),
          ],
        ),
        bottomNavigationBar: NavigationBar(
          selectedIndex: _currentIndex,
          onDestinationSelected: (i) {
            setState(() => _currentIndex = i);
            context.go(_routes[i]);
          },
          destinations: [
            NavigationDestination(
              icon: const Icon(Icons.home_outlined),
              selectedIcon: const Icon(Icons.home),
              label: l.homeTab,
            ),
            NavigationDestination(
              icon: const Icon(Icons.chat_bubble_outline),
              selectedIcon: const Icon(Icons.chat_bubble),
              label: l.messagesTab,
            ),
            NavigationDestination(
              icon: const Icon(Icons.add_circle_outline),
              selectedIcon: const Icon(Icons.add_circle),
              label: l.publishTab,
            ),
            NavigationDestination(
              icon: const Icon(Icons.person_outline),
              selectedIcon: const Icon(Icons.person),
              label: l.profileTab,
            ),
          ],
        ),
      ),
    );
  }
}
