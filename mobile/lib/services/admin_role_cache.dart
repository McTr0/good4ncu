import 'token_storage.dart';

class AdminRoleCache {
  AdminRoleCache._();

  static final AdminRoleCache instance = AdminRoleCache._();

  bool? _isAdmin;
  String? _token;

  bool? getCached(String token) {
    if (_token == token) {
      return _isAdmin;
    }
    return null;
  }

  void save(String token, bool isAdmin) {
    _token = token;
    _isAdmin = isAdmin;
  }

  Future<bool?> getCachedForCurrentToken() async {
    final token = await TokenStorage.instance.getAccessToken();
    if (token == null || token.isEmpty) {
      return null;
    }
    return getCached(token);
  }

  Future<void> saveForCurrentToken(bool isAdmin) async {
    final token = await TokenStorage.instance.getAccessToken();
    if (token == null || token.isEmpty) {
      invalidate();
      return;
    }
    save(token, isAdmin);
  }

  void invalidate() {
    _token = null;
    _isAdmin = null;
  }
}
