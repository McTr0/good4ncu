// Implementation for native platforms (iOS/Android) using dart:io.
import 'dart:io';

String getApiBaseUrl() {
  // Use localhost for iOS simulator, 10.0.2.2 for Android emulator
  if (Platform.isAndroid) {
    return 'http://10.0.2.2:3000';
  }
  // iOS simulator and other platforms use localhost
  return 'http://localhost:3000';
}

String getWsUrl() {
  if (Platform.isAndroid) {
    return 'ws://10.0.2.2:3000/api/ws';
  }
  return 'ws://localhost:3000/api/ws';
}
