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
import '../pages/trust_page.dart';
import '../services/token_storage.dart';
import '../services/ws_service.dart';

final GlobalKey<NavigatorState> _rootNavigatorKey = BaseService.navigatorKey;

Future<bool> _isLoggedIn() async {
  final token = await TokenStorage.instance.getAccessToken();
  return token != null && token.isNotEmpty;
}

final GoRouter appRouter = GoRouter(
  navigatorKey: _rootNavigatorKey,
  initialLocation: '/',
  redirect: (context, state) async {
    try {
      final loggedIn = await _isLoggedIn();
      final onAuthRoute = state.matchedLocation == '/login';
      if (!loggedIn && !onAuthRoute) {
        return '/login';
      }
      if (loggedIn && onAuthRoute) {
        // Connect global WS singleton on successful login redirect.
        WsService.instance.connect();
        return '/';
      }
      // On initial load when already logged in, ensure WS is connected.
      if (loggedIn) {
        WsService.instance.connect();
      }
    } catch (e) {
      // If auth check fails, redirect to login
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
    ShellRoute(
      builder: (context, state, child) => _ShellScaffold(child: child),
      routes: [
        GoRoute(
          path: '/',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: HomePage()),
        ),
        GoRoute(
          path: '/listing/:id',
          pageBuilder: (context, state) {
            final id = state.pathParameters['id']!;
            return NoTransitionPage(child: ListingDetailPage(listingId: id));
          },
        ),
        GoRoute(
          path: '/create',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: CreateListingPage()),
        ),
        GoRoute(
          path: '/my-listings',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: MyListingsPage()),
        ),
        GoRoute(
          path: '/profile',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: ProfilePage()),
        ),
        GoRoute(
          path: '/orders',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: MyOrdersPage()),
        ),
        GoRoute(
          path: '/orders/:id',
          pageBuilder: (context, state) {
            final id = state.pathParameters['id']!;
            return NoTransitionPage(child: OrderDetailPage(orderId: id));
          },
        ),
        GoRoute(
          path: '/chat',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: ChatPage()),
        ),
        GoRoute(
          path: '/conversations',
          pageBuilder: (context, state) =>
              const NoTransitionPage(child: ConversationListPage()),
        ),
        GoRoute(
          path: '/chat/:conversationId',
          pageBuilder: (context, state) {
            final id = state.pathParameters['conversationId']!;
            final extra = state.extra as Map<String, dynamic>?;
            final otherUserId = extra?['otherUserId'] as String? ?? '';
            final otherUsername = extra?['otherUsername'] as String? ?? '';
            return NoTransitionPage(
              child: UserChatPage(
                conversationId: id,
                otherUserId: otherUserId,
                otherUsername: otherUsername,
              ),
            );
          },
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

  static const _routes = [
    '/',
    '/conversations',
    '/create',
    '/my-listings',
    '/profile',
  ];

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    if (l == null) {
      return const SizedBox.shrink(); // Guard against early context without localization
    }
    final location = GoRouterState.of(context).matchedLocation;
    final idx = _routes.indexOf(location);
    if (idx >= 0) {
      _currentIndex = idx;
    }

    return Scaffold(
      body: widget.child,
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
            icon: const Icon(Icons.inventory_2_outlined),
            selectedIcon: const Icon(Icons.inventory_2),
            label: l.myListingsTab,
          ),
          NavigationDestination(
            icon: const Icon(Icons.person_outline),
            selectedIcon: const Icon(Icons.person),
            label: l.profileTab,
          ),
        ],
      ),
    );
  }
}
