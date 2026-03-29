export 'ws_channel_stub.dart'
    if (dart.library.io) 'ws_channel_io.dart'
    if (dart.library.html) 'ws_channel_web.dart';
