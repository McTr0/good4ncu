import 'dart:convert';
import 'package:flutter/material.dart';
import '../l10n/app_localizations.dart';
import 'package:image_picker/image_picker.dart';
import 'package:record/record.dart';
import 'package:path_provider/path_provider.dart';
import 'dart:io';
import '../services/api_service.dart';
import '../models/models.dart';

class ChatPage extends StatefulWidget {
  const ChatPage({super.key});

  @override
  State<ChatPage> createState() => _ChatPageState();
}

class _ChatPageState extends State<ChatPage> {
  final TextEditingController _controller = TextEditingController();
  final List<ChatMessage> _messages = [];
  final ApiService _apiService = ApiService();
  final ImagePicker _picker = ImagePicker();
  final AudioRecorder _audioRecorder = AudioRecorder();

  String? _selectedImageBase64;
  String? _selectedAudioBase64;
  bool _isRecording = false;
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    // Add initial AI greeting
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final l = AppLocalizations.of(context)!;
      setState(() {
        _messages.add(ChatMessage(
          sender: 'bot',
          content: l.aiGreeting,
          timestamp: DateTime.now(),
        ));
      });
    });
  }

  @override
  void dispose() {
    _audioRecorder.dispose();
    super.dispose();
  }

  Future<void> _pickImage() async {
    final XFile? image = await _picker.pickImage(
      source: ImageSource.gallery,
      imageQuality: 50,
      maxWidth: 1024,
    );
    if (image != null) {
      final bytes = await image.readAsBytes();
      setState(() {
        _selectedImageBase64 = base64Encode(bytes);
      });
    }
  }

  Future<void> _toggleRecording() async {
    if (_isRecording) {
      final path = await _audioRecorder.stop();
      if (path != null) {
        final bytes = await File(path).readAsBytes();
        setState(() {
          _isRecording = false;
          _selectedAudioBase64 = base64Encode(bytes);
        });
        _sendMessage();
      }
    } else {
      if (await _audioRecorder.hasPermission()) {
        final directory = await getTemporaryDirectory();
        final path = '${directory.path}/audio_${DateTime.now().millisecondsSinceEpoch}.ogg';
        await _audioRecorder.start(const RecordConfig(), path: path);
        setState(() => _isRecording = true);
      }
    }
  }

  Future<void> _sendMessage() async {
    final text = _controller.text.trim();
    if (text.isEmpty && _selectedImageBase64 == null && _selectedAudioBase64 == null) return;

    final userMsg = ChatMessage(
      sender: 'user',
      content: text.isEmpty ? '[Multimedia Message]' : text,
      imageBase64: _selectedImageBase64,
      audioBase64: _selectedAudioBase64,
      timestamp: DateTime.now(),
    );

    setState(() {
      _messages.add(userMsg);
      _isLoading = true;
      _controller.clear();
      _selectedImageBase64 = null;
      _selectedAudioBase64 = null;
    });

    try {
      final reply = await _apiService.sendChatMessage(userMsg);
      setState(() {
        _messages.add(ChatMessage(
          sender: 'bot',
          content: reply,
          timestamp: DateTime.now(),
        ));
      });
    } catch (e) {
      final l = AppLocalizations.of(context)!;
      setState(() {
        _messages.add(ChatMessage(
          sender: 'bot',
          content: '${l.aiError}: $e',
          timestamp: DateTime.now(),
        ));
      });
    } finally {
      setState(() => _isLoading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Column(
      children: [
        Expanded(
          child: ListView.builder(
            padding: const EdgeInsets.all(16),
            itemCount: _messages.length + (_isLoading ? 1 : 0),
            itemBuilder: (context, index) {
              if (index == _messages.length) {
                return const Align(
                  alignment: Alignment.centerLeft,
                  child: Padding(
                    padding: EdgeInsets.all(8.0),
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                );
              }
              final msg = _messages[index];
              final isUser = msg.sender == 'user';
              return Align(
                alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
                child: Container(
                  constraints: BoxConstraints(
                    maxWidth: MediaQuery.of(context).size.width * 0.75,
                  ),
                  margin: const EdgeInsets.symmetric(vertical: 4),
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: isUser ? const Color(0xFF6366F1) : Colors.grey[200],
                    borderRadius: BorderRadius.circular(16).copyWith(
                      bottomRight: isUser ? const Radius.circular(0) : const Radius.circular(16),
                      bottomLeft: !isUser ? const Radius.circular(0) : const Radius.circular(16),
                    ),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      if (msg.imageBase64 != null)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 8.0),
                          child: ClipRRect(
                            borderRadius: BorderRadius.circular(8),
                            child: Image.memory(base64Decode(msg.imageBase64!)),
                          ),
                        ),
                      if (msg.audioBase64 != null)
                        Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            const Icon(Icons.mic, size: 16),
                            const SizedBox(width: 4),
                            Text('Voice Message', style: TextStyle(fontSize: 12, color: isUser ? Colors.white : Colors.black87)),
                          ],
                        ),
                      Text(
                        msg.content,
                        style: TextStyle(
                          color: isUser ? Colors.white : Colors.black87,
                          fontSize: 16,
                        ),
                        softWrap: true,
                      ),
                    ],
                  ),
                ),
              );
            },
          ),
        ),
        if (_selectedImageBase64 != null)
          Container(
            padding: const EdgeInsets.all(8),
            height: 100,
            child: Stack(
              children: [
                Image.memory(base64Decode(_selectedImageBase64!)),
                Positioned(
                  right: 0,
                  top: 0,
                  child: IconButton(
                    icon: const Icon(Icons.close, color: Colors.red),
                    onPressed: () => setState(() => _selectedImageBase64 = null),
                  ),
                ),
              ],
            ),
          ),
        if (_isRecording)
          Padding(
            padding: const EdgeInsets.all(8.0),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                const Icon(Icons.circle, color: Colors.red, size: 12),
                const SizedBox(width: 8),
                Text('Recording...', style: TextStyle(color: Colors.red, fontWeight: FontWeight.bold)),
              ],
            ),
          ),
        Padding(
          padding: const EdgeInsets.all(8.0),
          child: Row(
            children: [
              IconButton(icon: const Icon(Icons.image), onPressed: _pickImage),
              IconButton(
                icon: Icon(_isRecording ? Icons.stop : Icons.mic, color: _isRecording ? Colors.red : null),
                onPressed: _toggleRecording,
              ),
              Expanded(
                child: TextField(
                  controller: _controller,
                  decoration: InputDecoration(
                    hintText: l.typeMessage,
                    border: const OutlineInputBorder(borderRadius: BorderRadius.all(Radius.circular(24))),
                    contentPadding: const EdgeInsets.symmetric(horizontal: 16),
                  ),
                ),
              ),
              IconButton(icon: const Icon(Icons.send, color: Color(0xFF6366F1)), onPressed: _sendMessage),
            ],
          ),
        ),
      ],
    );
  }
}
