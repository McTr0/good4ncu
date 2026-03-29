import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Native (iOS / Android) implementation backed by FlutterSecureStorage.
const FlutterSecureStorage _storage = FlutterSecureStorage(
  aOptions: AndroidOptions(encryptedSharedPreferences: true),
  iOptions: IOSOptions(),
);

Future<String?> secureRead(String key) => _storage.read(key: key);
Future<void> secureWrite(String key, String value) =>
    _storage.write(key: key, value: value);
Future<void> secureDelete(String key) => _storage.delete(key: key);
