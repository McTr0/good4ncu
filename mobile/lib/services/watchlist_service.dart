import 'base_service.dart';
import '../models/models.dart';

/// Watchlist service — handles user's saved listings.
class WatchlistService extends BaseService {
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

  /// Get user's watchlist.
  /// GET /api/watchlist
  Future<WatchlistResponse> getWatchlist({
    int limit = 20,
    int offset = 0,
  }) async {
    final headers = await authHeaders();
    final uri = _apiUri(
      ['api', 'watchlist'],
      queryParameters: {'limit': '$limit', 'offset': '$offset'},
    );
    final response = await get(uri, headers);
    return handleResponse(
      response,
      (data) => WatchlistResponse.fromJson(data as Map<String, dynamic>),
    );
  }

  /// Add listing to watchlist.
  /// POST /api/watchlist/{listingId}
  Future<void> addToWatchlist(String listingId) async {
    final headers = await authHeaders();
    final response = await post(
      _apiUri(['api', 'watchlist', listingId]),
      headers,
      '{}',
    );
    handleResponse(response, (_) {});
  }

  /// Remove listing from watchlist.
  /// DELETE /api/watchlist/{listingId}
  Future<void> removeFromWatchlist(String listingId) async {
    final headers = await authHeaders();
    final response = await delete(
      _apiUri(['api', 'watchlist', listingId]),
      headers,
    );
    handleResponse(response, (_) {});
  }

  /// Check if a listing is in the watchlist.
  /// GET /api/watchlist/{listingId}
  Future<bool> isWatched(String listingId) async {
    final headers = await authHeaders();
    final response = await get(
      _apiUri(['api', 'watchlist', listingId]),
      headers,
    );
    final data = handleResponse(response, (d) => d as Map<String, dynamic>);
    return data['watched'] ?? false;
  }
}
