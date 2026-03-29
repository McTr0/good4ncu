import 'package:shared_preferences/shared_preferences.dart';

enum NotificationFilterPreference { all, unread }

abstract class NotificationFilterPreferenceStore {
  Future<String?> readString(String key);

  Future<bool> writeString(String key, String value);
}

class SharedPreferencesNotificationFilterPreferenceStore
    implements NotificationFilterPreferenceStore {
  @override
  Future<String?> readString(String key) async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(key);
  }

  @override
  Future<bool> writeString(String key, String value) async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.setString(key, value);
  }
}

abstract class NotificationFilterStorage {
  Future<NotificationFilterPreference> readFilter();

  Future<void> writeFilter(NotificationFilterPreference filter);
}

class SharedPrefsNotificationFilterStorage
    implements NotificationFilterStorage {
  SharedPrefsNotificationFilterStorage({
    NotificationFilterPreferenceStore? store,
  }) : _store = store ?? SharedPreferencesNotificationFilterPreferenceStore();

  static const String _filterKey = 'notifications_filter';
  static const String _all = 'all';
  static const String _unread = 'unread';
  final NotificationFilterPreferenceStore _store;

  @override
  Future<NotificationFilterPreference> readFilter() async {
    final stored = await _store.readString(_filterKey);
    if (stored == _unread) {
      return NotificationFilterPreference.unread;
    }
    return NotificationFilterPreference.all;
  }

  @override
  Future<void> writeFilter(NotificationFilterPreference filter) async {
    final value = filter == NotificationFilterPreference.unread
        ? _unread
        : _all;
    final persisted = await _store.writeString(_filterKey, value);
    if (!persisted) {
      throw Exception('failed to persist notifications filter preference');
    }
  }
}
