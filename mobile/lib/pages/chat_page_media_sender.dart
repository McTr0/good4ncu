import 'dart:typed_data';

import 'package:image_picker/image_picker.dart';

import '../services/upload_service.dart';

class ChatPageMediaUploadException implements Exception {
  ChatPageMediaUploadException(this.message);

  final String message;

  @override
  String toString() => message;
}

class ChatPageUploadedMedia {
  const ChatPageUploadedMedia({this.imageUrl, this.audioUrl});

  final String? imageUrl;
  final String? audioUrl;
}

class ChatPageMediaSender {
  ChatPageMediaSender({UploadService? uploadService})
    : _uploadService = uploadService ?? UploadService();

  final UploadService _uploadService;

  Future<ChatPageUploadedMedia> uploadSelectedMedia({
    XFile? pickedImage,
    Uint8List? imageBytes,
    List<int>? audioBytes,
  }) async {
    String? imageUrl;
    String? audioUrl;

    if (pickedImage != null) {
      final resolvedImageBytes = imageBytes ?? await pickedImage.readAsBytes();
      imageUrl = await uploadPickedImage(
        pickedImage,
        imageBytes: resolvedImageBytes,
      );
    }

    if (audioBytes != null && audioBytes.isNotEmpty) {
      audioUrl = await uploadAudioBytes(audioBytes);
    }

    return ChatPageUploadedMedia(imageUrl: imageUrl, audioUrl: audioUrl);
  }

  Future<String> uploadPickedImage(
    XFile pickedImage, {
    Uint8List? imageBytes,
  }) async {
    final resolvedImageBytes = imageBytes ?? await pickedImage.readAsBytes();

    try {
      final extension = inferImageExtension(pickedImage.path);
      final contentType = contentTypeForImageExtension(extension);
      return await _uploadService.uploadImageBytes(
        resolvedImageBytes,
        extension: extension,
        contentType: contentType,
      );
    } catch (_) {
      throw ChatPageMediaUploadException('图片上传失败，请重试');
    }
  }

  Future<String> uploadAudioBytes(List<int> audioBytes) async {
    try {
      return await _uploadService.uploadAudioBytes(audioBytes);
    } catch (_) {
      throw ChatPageMediaUploadException('语音上传失败，请重试');
    }
  }

  String inferImageExtension(String path) {
    final lower = path.toLowerCase();
    if (lower.endsWith('.png')) return 'png';
    if (lower.endsWith('.webp')) return 'webp';
    return 'jpg';
  }

  String contentTypeForImageExtension(String extension) {
    switch (extension) {
      case 'png':
        return 'image/png';
      case 'webp':
        return 'image/webp';
      default:
        return 'image/jpeg';
    }
  }
}
