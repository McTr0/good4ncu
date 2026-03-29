import 'package:shared_preferences/shared_preferences.dart';

enum NotificationFilterPreference { all, unread }

abstract class NotificationFilterStorage {
  Future<NotificationFilterPreference> readFilter();

  Future<void> writeFilter(NotificationFilterPreference filter);
}

class SharedPrefsNotificationFilterStorage
    implements NotificationFilterStorage {
  static const String _filterKey = 'notifications_filter';
  static const String _all = 'all';
  static const String _unread = 'unread';

  @override
  Future<NotificationFilterPreference> readFilter() async {
    final prefs = await SharedPreferences.getInstance();
    final stored = prefs.getString(_filterKey);
    if (stored == _unread) {
      return NotificationFilterPreference.unread;
    }
    return NotificationFilterPreference.all;
  }

  @override
  Future<void> writeFilter(NotificationFilterPreference filter) async {
    final prefs = await SharedPreferences.getInstance();
    final value = filter == NotificationFilterPreference.unread
        ? _unread
        : _all;
    final persisted = await prefs.setString(_filterKey, value);
    if (!persisted) {
      throw Exception('failed to persist notifications filter preference');
    }
  }
}
