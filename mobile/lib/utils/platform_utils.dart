// Platform-aware URL helper.
//
// Uses conditional imports to select the right implementation:
// - platform_utils_io.dart: for iOS/Android (uses Platform from dart:io)
// - platform_utils_web.dart: for web (uses kIsWeb from flutter/foundation)
// - platform_utils_stub.dart: fallback for unsupported platforms

export 'platform_utils_stub.dart'
    if (dart.library.io) 'platform_utils_io.dart'
    if (dart.library.html) 'platform_utils_web.dart';
