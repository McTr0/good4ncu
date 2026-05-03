import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:record/record.dart';

import '../services/ws_service.dart';
import 'user_chat_media_sender.dart';

abstract class UserChatAudioRecorder {
  Future<bool> hasPermission();

  Future<void> start(String path);

  Future<String?> stop();

  Future<void> dispose();
}

class DeviceUserChatAudioRecorder implements UserChatAudioRecorder {
  DeviceUserChatAudioRecorder({AudioRecorder? recorder})
    : _recorder = recorder ?? AudioRecorder();

  final AudioRecorder _recorder;

  @override
  Future<bool> hasPermission() => _recorder.hasPermission();

  @override
  Future<void> start(String path) {
    return _recorder.start(
      const RecordConfig(encoder: AudioEncoder.opus),
      path: path,
    );
  }

  @override
  Future<String?> stop() => _recorder.stop();

  @override
  Future<void> dispose() => _recorder.dispose();
}

abstract class UserChatNotificationSource {
  Stream<WsNotification> get stream;
}

class WsUserChatNotificationSource implements UserChatNotificationSource {
  const WsUserChatNotificationSource();

  @override
  Stream<WsNotification> get stream => WsService.instance.stream;
}

typedef UserChatTempDirectoryPath = Future<String> Function();

Future<String> defaultUserChatTempDirectoryPath() async {
  final directory = await getTemporaryDirectory();
  return directory.path;
}

class UserChatSessionController extends ChangeNotifier {
  UserChatSessionController({
    UserChatMediaSender? mediaSender,
    UserChatAudioRecorder? audioRecorder,
    UserChatNotificationSource? notificationSource,
    UserChatTempDirectoryPath? tempDirectoryPath,
  }) : _mediaSender = mediaSender ?? UserChatMediaSender(),
       _audioRecorder = audioRecorder ?? DeviceUserChatAudioRecorder(),
       _notificationSource =
           notificationSource ?? const WsUserChatNotificationSource(),
       _tempDirectoryPath =
           tempDirectoryPath ?? defaultUserChatTempDirectoryPath;

  final UserChatMediaSender _mediaSender;
  final UserChatAudioRecorder _audioRecorder;
  final UserChatNotificationSource _notificationSource;
  final UserChatTempDirectoryPath _tempDirectoryPath;

  Timer? _recordingTimer;
  StreamSubscription<WsNotification>? _wsSubscription;

  bool _isRecording = false;
  int _recordingSeconds = 0;

  bool get isRecording => _isRecording;

  int get recordingSeconds => _recordingSeconds;

  void connectWs(void Function(WsNotification notification) onNotification) {
    _wsSubscription?.cancel();
    _wsSubscription = _notificationSource.stream.listen(onNotification);
  }

  Future<void> toggleRecording({
    required bool Function() canSendMedia,
    required UserChatSendMessage sendMessage,
    required void Function(String message) onError,
  }) async {
    if (_isRecording) {
      await _stopRecording(
        canSendMedia: canSendMedia,
        sendMessage: sendMessage,
        onError: onError,
      );
      return;
    }

    if (!await _audioRecorder.hasPermission()) {
      return;
    }

    final directoryPath = await _tempDirectoryPath();
    final path =
        '$directoryPath/audio_${DateTime.now().millisecondsSinceEpoch}.ogg';
    await _audioRecorder.start(path);
    _isRecording = true;
    _recordingSeconds = 0;
    notifyListeners();

    _recordingTimer?.cancel();
    _recordingTimer = Timer.periodic(const Duration(seconds: 1), (timer) {
      _recordingSeconds++;
      notifyListeners();
      if (_recordingSeconds >= 60) {
        _stopRecording(
          canSendMedia: canSendMedia,
          sendMessage: sendMessage,
          onError: onError,
        );
      }
    });
  }

  Future<void> _stopRecording({
    required bool Function() canSendMedia,
    required UserChatSendMessage sendMessage,
    required void Function(String message) onError,
  }) async {
    _recordingTimer?.cancel();
    _recordingTimer = null;

    final path = await _audioRecorder.stop();
    _isRecording = false;
    notifyListeners();

    if (path == null) {
      return;
    }

    final bytes = await File(path).readAsBytes();
    if (!canSendMedia()) {
      onError('等待连接建立后再发送消息');
      return;
    }

    try {
      await _mediaSender.sendAudioBytes(bytes, sendMessage: sendMessage);
    } on UserChatMediaSendException catch (e) {
      onError(e.message);
    }
  }

  @override
  void dispose() {
    _recordingTimer?.cancel();
    _wsSubscription?.cancel();
    _audioRecorder.dispose();
    super.dispose();
  }
}
