import 'package:flutter_test/flutter_test.dart';
import 'package:good4ncu_mobile/services/notification_filter_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  group('SharedPrefsNotificationFilterStorage', () {
    late SharedPrefsNotificationFilterStorage storage;

    setUp(() {
      SharedPreferences.setMockInitialValues(<String, Object>{});
      storage = SharedPrefsNotificationFilterStorage();
    });

    test('defaults to all when no persisted value exists', () async {
      final filter = await storage.readFilter();

      expect(filter, NotificationFilterPreference.all);
    });

    test('maps unread persisted value to unread preference', () async {
      SharedPreferences.setMockInitialValues(<String, Object>{
        'notifications_filter': 'unread',
      });

      final filter = await storage.readFilter();

      expect(filter, NotificationFilterPreference.unread);
    });

    test('falls back to all for unknown persisted value', () async {
      SharedPreferences.setMockInitialValues(<String, Object>{
        'notifications_filter': 'unexpected-value',
      });

      final filter = await storage.readFilter();

      expect(filter, NotificationFilterPreference.all);
    });

    test('writes unread preference to shared preferences', () async {
      await storage.writeFilter(NotificationFilterPreference.unread);
      final prefs = await SharedPreferences.getInstance();

      expect(prefs.getString('notifications_filter'), 'unread');
    });

    test('writes all preference to shared preferences', () async {
      await storage.writeFilter(NotificationFilterPreference.all);
      final prefs = await SharedPreferences.getInstance();

      expect(prefs.getString('notifications_filter'), 'all');
    });
  });
}
