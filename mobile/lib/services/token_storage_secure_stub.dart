// Stub used on Web platform — these functions are never actually called
// because TokenStorage checks web first, but they must exist to
// satisfy the conditional-import contract.

Future<String?> secureRead(String key) async => null;
Future<void> secureWrite(String key, String value) async {}
Future<void> secureDelete(String key) async {}
