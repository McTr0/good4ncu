import '../models/models.dart';
import 'base_service.dart';

/// Notification service for system messages and unread state.
class NotificationService extends BaseService {
  Uri _apiUri(
    List<String> extraPathSegments, {
    Map<String, String>? queryParameters,
  }) {
    final base = Uri.parse(baseUrl);
    final segments = [
      ...base.pathSegments.where((segment) => segment.isNotEmpty),
      ...extraPathSegments,
    ];
    return base.replace(
      pathSegments: segments,
      queryParameters: queryParameters,
    );
  }

  /// Get notifications for current user.
  /// GET /api/notifications
  Future<NotificationsResponse> getNotifications({
    int limit = 20,
    int offset = 0,
    bool includeRead = true,
  }) async {
    final headers = await authHeaders();
    final uri = _apiUri(
      ['api', 'notifications'],
      queryParameters: {
        'limit': '$limit',
        'offset': '$offset',
        'include_read': '$includeRead',
      },
    );
    final response = await get(uri, headers);
    return handleResponse(
      response,
      (data) => NotificationsResponse.fromJson(data as Map<String, dynamic>),
    );
  }

  /// Mark a notification as read.
  /// POST /api/notifications/{id}/read
  Future<void> markNotificationRead(String notificationId) async {
    final headers = await authHeaders();
    final response = await post(
      _apiUri(['api', 'notifications', notificationId, 'read']),
      headers,
      '{}',
    );
    handleResponse(response, (_) {});
  }

  /// Mark all notifications as read.
  /// POST /api/notifications/read-all
  Future<int> markAllRead() async {
    final headers = await authHeaders();
    final response = await post(
      _apiUri(['api', 'notifications', 'read-all']),
      headers,
      '{}',
    );
    final data = handleResponse(response, (d) => d as Map<String, dynamic>);
    return data['marked_count'] ?? 0;
  }
}
