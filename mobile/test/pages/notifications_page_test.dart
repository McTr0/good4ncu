import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:good4ncu_mobile/l10n/app_localizations.dart';
import 'package:good4ncu_mobile/models/models.dart';
import 'package:good4ncu_mobile/pages/notifications_page.dart';
import 'package:good4ncu_mobile/services/notification_service.dart';

class _StubNotificationService extends NotificationService {
  _StubNotificationService({
    required this.onGetNotifications,
    this.onMarkAllRead,
  });

  final Future<NotificationsResponse> Function(
    int limit,
    int offset,
    bool includeRead,
  )
  onGetNotifications;
  final Future<int> Function()? onMarkAllRead;

  int markAllReadCalls = 0;
  final List<int> requestedOffsets = [];
  final List<bool> requestedIncludeRead = [];
  final List<String> markedReadIds = [];

  @override
  Future<NotificationsResponse> getNotifications({
    int limit = 20,
    int offset = 0,
    bool includeRead = true,
  }) {
    requestedOffsets.add(offset);
    requestedIncludeRead.add(includeRead);
    return onGetNotifications(limit, offset, includeRead);
  }

  @override
  Future<int> markAllRead() async {
    markAllReadCalls += 1;
    if (onMarkAllRead != null) {
      return onMarkAllRead!();
    }
    return 0;
  }

  @override
  Future<void> markNotificationRead(String notificationId) async {
    markedReadIds.add(notificationId);
  }
}

Widget _buildTestApp(Widget child) {
  return MaterialApp(
    locale: const Locale('en'),
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: child,
  );
}

void main() {
  group('NotificationsPage', () {
    testWidgets('shows notifications and mark all read action works', (
      tester,
    ) async {
      final service = _StubNotificationService(
        onGetNotifications: (limit, offset, includeRead) async =>
            NotificationsResponse(
              items: const [
                AppNotification(
                  id: 'n1',
                  eventType: 'new_message',
                  title: 'New message',
                  body: 'You have a new message',
                  isRead: false,
                  createdAt: '2026-03-01T09:00:00Z',
                ),
              ],
              total: 1,
              unreadCount: 1,
              limit: 20,
              offset: 0,
            ),
        onMarkAllRead: () async => 1,
      );

      await tester.pumpWidget(
        _buildTestApp(NotificationsPage(notificationService: service)),
      );
      await tester.pumpAndSettle();
      final l = AppLocalizations.of(tester.element(find.byType(Scaffold)))!;

      expect(find.text('New message'), findsOneWidget);
      expect(find.text(l.markAllRead), findsOneWidget);

      await tester.tap(find.text(l.markAllRead));
      await tester.pumpAndSettle();

      expect(service.markAllReadCalls, 1);
    });

    testWidgets('shows empty state when no notifications', (tester) async {
      final service = _StubNotificationService(
        onGetNotifications: (limit, offset, includeRead) async =>
            const NotificationsResponse(
              items: [],
              total: 0,
              unreadCount: 0,
              limit: 20,
              offset: 0,
            ),
      );

      await tester.pumpWidget(
        _buildTestApp(NotificationsPage(notificationService: service)),
      );
      await tester.pumpAndSettle();
      final l = AppLocalizations.of(tester.element(find.byType(Scaffold)))!;

      expect(find.text(l.notificationsEmpty), findsOneWidget);
    });

    testWidgets('tapping unread item marks notification as read', (
      tester,
    ) async {
      final service = _StubNotificationService(
        onGetNotifications: (limit, offset, includeRead) async =>
            const NotificationsResponse(
              items: [
                AppNotification(
                  id: 'n1',
                  eventType: 'new_message',
                  title: 'New message',
                  body: 'Tap to read',
                  isRead: false,
                  createdAt: '2026-03-01T09:00:00Z',
                ),
              ],
              total: 1,
              unreadCount: 1,
              limit: 20,
              offset: 0,
            ),
      );

      await tester.pumpWidget(
        _buildTestApp(NotificationsPage(notificationService: service)),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.text('New message'));
      await tester.pumpAndSettle();

      expect(service.markedReadIds, ['n1']);
    });

    testWidgets('loads next page when tapping Load more', (tester) async {
      final service = _StubNotificationService(
        onGetNotifications: (limit, offset, includeRead) async {
          if (offset == 0) {
            return const NotificationsResponse(
              items: [
                AppNotification(
                  id: 'n1',
                  eventType: 'new_message',
                  title: 'New message',
                  body: 'Page 1',
                  isRead: true,
                  createdAt: '2026-03-01T09:00:00Z',
                ),
              ],
              total: 2,
              unreadCount: 0,
              limit: 20,
              offset: 0,
            );
          }
          return const NotificationsResponse(
            items: [
              AppNotification(
                id: 'n2',
                eventType: 'order_paid',
                title: 'Order paid',
                body: 'Page 2',
                isRead: true,
                createdAt: '2026-03-02T09:00:00Z',
              ),
            ],
            total: 2,
            unreadCount: 0,
            limit: 20,
            offset: 1,
          );
        },
      );

      await tester.pumpWidget(
        _buildTestApp(NotificationsPage(notificationService: service)),
      );
      await tester.pumpAndSettle();
      final l = AppLocalizations.of(tester.element(find.byType(Scaffold)))!;

      expect(find.text(l.loadMore), findsOneWidget);

      await tester.tap(find.text(l.loadMore));
      await tester.pumpAndSettle();

      expect(service.requestedOffsets, [0, 1]);
      expect(find.text('Order paid'), findsOneWidget);
    });

    testWidgets('toggle filter switches include_read to false', (tester) async {
      final service = _StubNotificationService(
        onGetNotifications: (limit, offset, includeRead) async =>
            const NotificationsResponse(
              items: [
                AppNotification(
                  id: 'n1',
                  eventType: 'new_message',
                  title: 'New message',
                  body: 'Filter test',
                  isRead: false,
                  createdAt: '2026-03-01T09:00:00Z',
                ),
              ],
              total: 1,
              unreadCount: 1,
              limit: 20,
              offset: 0,
            ),
      );

      await tester.pumpWidget(
        _buildTestApp(NotificationsPage(notificationService: service)),
      );
      await tester.pumpAndSettle();
      final l = AppLocalizations.of(tester.element(find.byType(Scaffold)))!;

      expect(service.requestedIncludeRead, [true]);
      expect(find.text(l.unreadOnly), findsOneWidget);

      await tester.tap(find.text(l.unreadOnly));
      await tester.pumpAndSettle();

      expect(service.requestedIncludeRead, [true, false]);
      expect(find.text(l.allNotifications), findsOneWidget);
    });

    testWidgets('in unread mode tapping item removes it from list', (
      tester,
    ) async {
      final service = _StubNotificationService(
        onGetNotifications: (limit, offset, includeRead) async =>
            NotificationsResponse(
              items: const [
                AppNotification(
                  id: 'n1',
                  eventType: 'new_message',
                  title: 'Unread item',
                  body: 'Tap should remove',
                  isRead: false,
                  createdAt: '2026-03-01T09:00:00Z',
                ),
              ],
              total: 1,
              unreadCount: 1,
              limit: 20,
              offset: 0,
            ),
      );

      await tester.pumpWidget(
        _buildTestApp(NotificationsPage(notificationService: service)),
      );
      await tester.pumpAndSettle();
      final l = AppLocalizations.of(tester.element(find.byType(Scaffold)))!;

      await tester.tap(find.text(l.unreadOnly));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Unread item'));
      await tester.pumpAndSettle();

      expect(service.markedReadIds, contains('n1'));
      expect(find.text('Unread item'), findsNothing);
      expect(find.text(l.notificationsEmpty), findsOneWidget);
    });

    testWidgets('failed filter switch rolls back to previous filter', (
      tester,
    ) async {
      int unreadAttempts = 0;
      int allAttempts = 0;
      final service = _StubNotificationService(
        onGetNotifications: (limit, offset, includeRead) async {
          if (!includeRead) {
            unreadAttempts += 1;
            throw Exception('unread request failed');
          }
          allAttempts += 1;
          if (allAttempts > 1) {
            throw Exception('should not need second all reload');
          }
          return const NotificationsResponse(
            items: [
              AppNotification(
                id: 'n1',
                eventType: 'new_message',
                title: 'Stable item',
                body: 'all list payload',
                isRead: true,
                createdAt: '2026-03-01T09:00:00Z',
              ),
            ],
            total: 1,
            unreadCount: 0,
            limit: 20,
            offset: 0,
          );
        },
      );

      await tester.pumpWidget(
        _buildTestApp(NotificationsPage(notificationService: service)),
      );
      await tester.pumpAndSettle();
      final l = AppLocalizations.of(tester.element(find.byType(Scaffold)))!;

      expect(find.text(l.unreadOnly), findsOneWidget);

      await tester.tap(find.text(l.unreadOnly));
      await tester.pumpAndSettle();

      expect(unreadAttempts, 1);
      expect(allAttempts, 1);
      expect(service.requestedIncludeRead, [true, false]);
      expect(find.text(l.unreadOnly), findsOneWidget);
      expect(find.text('Stable item'), findsOneWidget);
    });
  });
}
