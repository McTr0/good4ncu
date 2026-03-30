import 'dart:async';
import 'dart:convert';
import 'package:image_picker/image_picker.dart';
import 'package:flutter/material.dart';
import 'package:record/record.dart';
import 'package:path_provider/path_provider.dart';
import 'package:uuid/uuid.dart';
import 'dart:io';
import '../models/models.dart';
import '../services/api_service.dart';
import '../services/upload_service.dart';
import '../services/ws_service.dart';
import '../theme/app_theme.dart';
import '../components/audio_message_player.dart';

/// 私聊页面
class UserChatPage extends StatefulWidget {
  final String conversationId;
  final String otherUserId;
  final String otherUsername;

  const UserChatPage({
    super.key,
    required this.conversationId,
    required this.otherUserId,
    required this.otherUsername,
  });

  @override
  State<UserChatPage> createState() => _UserChatPageState();
}

class _UserChatPageState extends State<UserChatPage> {
  final ApiService _apiService = ApiService();
  final UploadService _uploadService = UploadService();
  final ImagePicker _imagePicker = ImagePicker();
  final TextEditingController _textController = TextEditingController();
  final ScrollController _scrollController = ScrollController();
  final AudioRecorder _audioRecorder = AudioRecorder();

  List<ConversationMessage> _messages = [];
  String? _currentUserId;
  bool _isLoading = true;
  bool _isSending = false;
  String? _error;

  /// 连接状态: null=无连接, 'connecting'=连接中, 'connected'=已连接
  String? _connectionStatus;

  /// 对方正在输入状态
  bool _isOtherTyping = false;
  Timer? _typingTimer;

  /// 录音状态
  bool _isRecording = false;
  int _recordingSeconds = 0;
  Timer? _recordingTimer;

  /// 正在编辑的消息ID
  String? _editingMessageId;

  StreamSubscription? _wsSubscription;

  @override
  void initState() {
    super.initState();
    _loadCurrentUser();
    _loadMessages();
    _connectWs();
    _markConnectionAsRead();
  }

  @override
  void dispose() {
    _textController.dispose();
    _scrollController.dispose();
    _wsSubscription?.cancel();
    _typingTimer?.cancel();
    _recordingTimer?.cancel();
    _audioRecorder.dispose();
    super.dispose();
  }

  Future<void> _loadCurrentUser() async {
    try {
      final profile = await _apiService.getUserProfile();
      setState(() {
        _currentUserId = profile['user_id']?.toString();
      });
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('加载失败: $e'), backgroundColor: Colors.red),
      );
    }
  }

  Future<void> _loadMessages() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final messages = await _apiService.getChatConversationMessages(
        widget.conversationId,
      );
      if (!mounted) return;
      setState(() {
        _messages = messages.reversed.toList();
        _isLoading = false;
      });
      _scrollToBottom();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _isLoading = false;
      });
    }
  }

  Future<void> _markConnectionAsRead() async {
    await _apiService
        .markConnectionAsRead(widget.conversationId)
        .catchError((_) {});
  }

  Future<void> _connectWs() async {
    _wsSubscription = WsService.instance.stream.listen(_handleWsNotification);
  }

  void _handleWsNotification(WsNotification notif) {
    if (!mounted) return;

    switch (notif.eventType) {
      case 'connection_established':
        setState(() => _connectionStatus = 'connected');
        _loadMessages();
        _showSnackBar('连接已建立');
        break;

      case 'connection_rejected':
        if (notif.connectionId == widget.conversationId) {
          setState(() => _connectionStatus = 'rejected');
          _showSnackBar('连接已被对方拒绝');
        }
        break;

      case 'new_message':
        final messageId = notif.messageId;
        if (messageId != null) {
          _apiService.markMessageRead(messageId).catchError((_) {});
          _loadMessages();
        }
        break;

      case 'message_read':
        _loadMessages();
        break;

      case 'typing':
        // 显示对方正在输入
        if (notif.conversationId == widget.conversationId &&
            notif.typingUserId != _currentUserId) {
          setState(() => _isOtherTyping = true);
          _typingTimer?.cancel();
          _typingTimer = Timer(const Duration(seconds: 3), () {
            if (mounted) setState(() => _isOtherTyping = false);
          });
        }
        break;

      case 'connection_request':
        _showConnectionRequestDialog(notif);
        break;
    }
  }

  void _showConnectionRequestDialog(WsNotification notif) {
    final connectionId = notif.connectionId;
    if (connectionId == null) return;

    showDialog(
      context: context,
      barrierDismissible: false,
      builder: (ctx) => AlertDialog(
        title: const Text('连接请求'),
        content: Text('${notif.title}\n\n${notif.body}\n\n确认后将开启消息已读功能'),
        actions: [
          TextButton(
            onPressed: () {
              Navigator.pop(ctx);
              _rejectConnection(connectionId);
            },
            child: const Text('拒绝'),
          ),
          ElevatedButton(
            onPressed: () {
              Navigator.pop(ctx);
              _acceptConnection(connectionId);
            },
            child: const Text('接受'),
          ),
        ],
      ),
    );
  }

  Future<void> _acceptConnection(String connectionId) async {
    try {
      await _apiService.acceptConnection(connectionId);
      setState(() => _connectionStatus = 'connected');
      _showSnackBar('已接受连接');
      _loadMessages();
    } catch (e) {
      _showSnackBar('接受失败: $e');
    }
  }

  Future<void> _rejectConnection(String connectionId) async {
    try {
      await _apiService.rejectConnection(connectionId);
      _showSnackBar('已拒绝连接');
    } catch (e) {
      _showSnackBar('拒绝失败: $e');
    }
  }

  /// 发送typing indicator
  void _sendTypingIndicator() {
    _apiService.sendTyping(widget.conversationId).catchError((_) {});
  }

  /// 开始编辑消息
  void _startEditMessage(ConversationMessage msg) {
    setState(() {
      _editingMessageId = msg.id;
      _textController.text = msg.content;
    });
  }

  /// 取消编辑
  void _cancelEdit() {
    setState(() {
      _editingMessageId = null;
      _textController.clear();
    });
  }

  /// 确认编辑
  Future<void> _confirmEdit() async {
    if (_editingMessageId == null || _textController.text.trim().isEmpty) {
      return;
    }

    final newContent = _textController.text.trim();
    try {
      final updated = await _apiService.editMessage(
        _editingMessageId!,
        newContent,
      );
      if (!mounted) return;
      setState(() {
        final idx = _messages.indexWhere((m) => m.id == _editingMessageId);
        if (idx >= 0) {
          _messages[idx] = updated;
        }
        _editingMessageId = null;
        _textController.clear();
      });
      _showSnackBar('消息已编辑');
    } catch (e) {
      if (!mounted) return;
      _showSnackBar('编辑失败: $e');
    }
  }

  Future<void> _sendMessage() async {
    if (_isSending || _textController.text.trim().isEmpty) return;

    if (_connectionStatus != 'connected') {
      _showSnackBar('等待连接建立后再发送消息');
      return;
    }

    // 如果正在编辑，先确认编辑
    if (_editingMessageId != null) {
      await _confirmEdit();
      return;
    }

    final text = _textController.text.trim();
    final tempMsg = ConversationMessage(
      id: 'temp_${const Uuid().v4()}',
      conversationId: widget.conversationId,
      senderId: _currentUserId ?? '',
      content: text,
      sentAt: DateTime.now(),
      status: 'sending',
    );

    setState(() {
      _messages.add(tempMsg);
      _isSending = true;
    });
    _textController.clear();
    _scrollToBottom();

    try {
      final reply = await _apiService.sendMessage(
        widget.conversationId,
        content: text,
      );
      if (!mounted) return;
      setState(() {
        final idx = _messages.indexWhere((m) => m.id == tempMsg.id);
        if (idx >= 0) {
          _messages[idx] = reply;
        }
        _isSending = false;
      });
      _scrollToBottom();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _messages.removeWhere((m) => m.id == tempMsg.id);
        _isSending = false;
      });
      _showSnackBar('发送失败: $e');
    }
  }

  Future<void> _pickAndSendImage() async {
    if (_connectionStatus != 'connected') {
      _showSnackBar('等待连接建立后再发送消息');
      return;
    }

    final picked = await _imagePicker.pickImage(
      source: ImageSource.gallery,
      imageQuality: 65,
      maxWidth: 1280,
    );
    if (picked == null) {
      return;
    }

    final bytes = await picked.readAsBytes();
    final imageBase64 = base64Encode(bytes);
    final tempMsg = ConversationMessage(
      id: 'temp_${const Uuid().v4()}',
      conversationId: widget.conversationId,
      senderId: _currentUserId ?? '',
      content: '[图片消息]',
      imageBase64: imageBase64,
      imageUrl: null,
      sentAt: DateTime.now(),
      status: 'sending',
    );

    setState(() {
      _messages.add(tempMsg);
      _isSending = true;
    });
    _scrollToBottom();

    try {
      String? uploadedImageUrl;
      try {
        final extension = _inferImageExtension(picked.path);
        final contentType = _contentTypeForImageExtension(extension);
        uploadedImageUrl = await _uploadService.uploadImageBytes(
          bytes,
          extension: extension,
          contentType: contentType,
        );
      } catch (_) {
        uploadedImageUrl = null;
      }

      final reply = await _apiService.sendMessage(
        widget.conversationId,
        content: '[图片消息]',
        imageBase64: uploadedImageUrl == null ? imageBase64 : null,
        imageUrl: uploadedImageUrl,
      );
      if (!mounted) return;
      setState(() {
        final idx = _messages.indexWhere((m) => m.id == tempMsg.id);
        if (idx >= 0) {
          _messages[idx] = reply;
        }
        _isSending = false;
      });
      _scrollToBottom();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _messages.removeWhere((m) => m.id == tempMsg.id);
        _isSending = false;
      });
      _showSnackBar('发送失败: $e');
    }
  }

  String _inferImageExtension(String path) {
    final lower = path.toLowerCase();
    if (lower.endsWith('.png')) return 'png';
    if (lower.endsWith('.webp')) return 'webp';
    return 'jpg';
  }

  String _contentTypeForImageExtension(String ext) {
    switch (ext) {
      case 'png':
        return 'image/png';
      case 'webp':
        return 'image/webp';
      default:
        return 'image/jpeg';
    }
  }

  /// 切换录音状态
  Future<void> _toggleRecording() async {
    if (_isRecording) {
      _recordingTimer?.cancel();
      final path = await _audioRecorder.stop();
      if (path != null) {
        final bytes = await File(path).readAsBytes();
        final audioBase64 = base64Encode(bytes);
        setState(() => _isRecording = false);
        await _sendAudioMessage(audioBase64, bytes);
      } else {
        setState(() => _isRecording = false);
      }
    } else {
      if (await _audioRecorder.hasPermission()) {
        final directory = await getTemporaryDirectory();
        final path =
            '${directory.path}/audio_${DateTime.now().millisecondsSinceEpoch}.ogg';
        await _audioRecorder.start(
          const RecordConfig(encoder: AudioEncoder.opus),
          path: path,
        );
        setState(() {
          _isRecording = true;
          _recordingSeconds = 0;
        });
        _recordingTimer = Timer.periodic(const Duration(seconds: 1), (timer) {
          if (mounted) {
            setState(() => _recordingSeconds++);
            if (_recordingSeconds >= 60) {
              _toggleRecording(); // 自动停止
            }
          }
        });
      }
    }
  }

  Future<void> _sendAudioMessage(
    String audioBase64,
    List<int> audioBytes,
  ) async {
    if (_connectionStatus != 'connected') {
      _showSnackBar('等待连接建立后再发送消息');
      return;
    }

    final tempMsg = ConversationMessage(
      id: 'temp_${const Uuid().v4()}',
      conversationId: widget.conversationId,
      senderId: _currentUserId ?? '',
      content: '[语音消息]',
      audioBase64: audioBase64,
      audioUrl: null,
      sentAt: DateTime.now(),
      status: 'sending',
    );

    setState(() {
      _messages.add(tempMsg);
      _isSending = true;
    });
    _scrollToBottom();

    try {
      String? uploadedAudioUrl;
      try {
        uploadedAudioUrl = await _uploadService.uploadAudioBytes(audioBytes);
      } catch (_) {
        uploadedAudioUrl = null;
      }

      final reply = await _apiService.sendMessage(
        widget.conversationId,
        content: '[语音消息]',
        audioBase64: uploadedAudioUrl == null ? audioBase64 : null,
        audioUrl: uploadedAudioUrl,
      );
      if (!mounted) return;
      setState(() {
        final idx = _messages.indexWhere((m) => m.id == tempMsg.id);
        if (idx >= 0) {
          _messages[idx] = reply;
        }
        _isSending = false;
      });
      _scrollToBottom();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _messages.removeWhere((m) => m.id == tempMsg.id);
        _isSending = false;
      });
      _showSnackBar('发送失败: $e');
    }
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 300),
          curve: Curves.easeOut,
        );
      }
    });
  }

  void _showSnackBar(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message), duration: const Duration(seconds: 2)),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: AppTheme.primary,
        foregroundColor: Colors.white,
        title: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            ConnectionIndicator(
              status: _connectionStatus,
              isWsConnected: WsService.instance.isConnected,
            ),
            const SizedBox(width: 8),
            Text(widget.otherUsername),
          ],
        ),
      ),
      body: Column(
        children: [
          if (_isOtherTyping)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              color: Colors.grey[100],
              child: Row(
                children: [
                  const SizedBox(
                    width: 12,
                    height: 12,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    '${widget.otherUsername} 正在输入...',
                    style: const TextStyle(color: Colors.grey, fontSize: 12),
                  ),
                ],
              ),
            ),
          Expanded(child: _buildMessageList()),
          _buildInputArea(),
        ],
      ),
    );
  }

  Widget _buildMessageList() {
    if (_isLoading && _messages.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null && _messages.isEmpty) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text('加载失败: $_error'),
            const SizedBox(height: 16),
            ElevatedButton(onPressed: _loadMessages, child: const Text('重试')),
          ],
        ),
      );
    }
    if (_messages.isEmpty) {
      return const Center(
        child: Text('暂无消息，开始聊天吧', style: TextStyle(color: Colors.grey)),
      );
    }
    return ListView.builder(
      controller: _scrollController,
      padding: const EdgeInsets.all(16),
      itemCount: _messages.length,
      itemBuilder: (context, index) {
        final msg = _messages[index];
        final isMe = msg.isFrom(_currentUserId ?? '');
        return MessageBubble(
          message: msg,
          isMe: isMe,
          isConnected: _connectionStatus == 'connected',
          onEdit: isMe && msg.canEdit ? () => _startEditMessage(msg) : null,
        );
      },
    );
  }

  Widget _buildInputArea() {
    if (_connectionStatus != 'connected') {
      return Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: Colors.orange.shade50,
          border: Border(top: BorderSide(color: Colors.orange.shade200)),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.hourglass_empty,
              color: Colors.orange.shade700,
              size: 18,
            ),
            const SizedBox(width: 8),
            Text('等待对方接受连接', style: TextStyle(color: Colors.orange.shade700)),
          ],
        ),
      );
    }

    // 录音中显示倒计时
    if (_isRecording) {
      return Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: Colors.red.shade50,
          border: Border(top: BorderSide(color: Colors.red.shade200)),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.circle, color: Colors.red, size: 12),
            const SizedBox(width: 8),
            Text(
              '录音中 ${_recordingSeconds}s / 60s',
              style: TextStyle(
                color: Colors.red.shade700,
                fontWeight: FontWeight.bold,
              ),
            ),
            const Spacer(),
            TextButton(onPressed: _toggleRecording, child: const Text('停止')),
          ],
        ),
      );
    }

    return Container(
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: Theme.of(context).cardTheme.color,
        border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: SafeArea(
        child: Row(
          children: [
            IconButton(
              icon: const Icon(Icons.image),
              onPressed: _isSending ? null : _pickAndSendImage,
            ),
            IconButton(
              icon: Icon(
                _isRecording ? Icons.stop : Icons.mic,
                color: _isRecording ? Colors.red : null,
              ),
              onPressed: _toggleRecording,
            ),
            if (_editingMessageId != null)
              IconButton(
                icon: const Icon(Icons.close, color: Colors.grey),
                onPressed: _cancelEdit,
              ),
            Expanded(
              child: TextField(
                controller: _textController,
                decoration: InputDecoration(
                  hintText: _editingMessageId != null ? '编辑消息...' : '输入消息...',
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(24),
                  ),
                  contentPadding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 8,
                  ),
                ),
                onChanged: (_) => _sendTypingIndicator(),
                onSubmitted: (_) => _sendMessage(),
              ),
            ),
            const SizedBox(width: 8),
            IconButton(
              icon: Icon(
                _editingMessageId != null ? Icons.check : Icons.send,
                color: _editingMessageId != null
                    ? Colors.green
                    : AppTheme.primary,
              ),
              onPressed: _editingMessageId != null
                  ? _confirmEdit
                  : _sendMessage,
            ),
          ],
        ),
      ),
    );
  }
}

class ConnectionIndicator extends StatefulWidget {
  final String? status;
  final bool isWsConnected;

  const ConnectionIndicator({
    super.key,
    this.status,
    this.isWsConnected = false,
  });

  @override
  State<ConnectionIndicator> createState() => ConnectionIndicatorState();
}

class ConnectionIndicatorState extends State<ConnectionIndicator>
    with SingleTickerProviderStateMixin {
  late AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 800),
    )..repeat(reverse: true);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    Color color;
    String label;
    Widget dot;

    // If WebSocket is not connected, always show offline (grey)
    if (!widget.isWsConnected) {
      color = Colors.grey;
      label = '离线';
      dot = Container(
        width: 8,
        height: 8,
        decoration: BoxDecoration(color: color, shape: BoxShape.circle),
      );
    } else {
      switch (widget.status) {
        case 'connected':
          color = AppTheme.success;
          label = '在线';
          dot = Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          );
          break;
        case 'pending':
          color = AppTheme.warning;
          label = '待接受';
          dot = Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          );
          break;
        case 'connecting':
          color = AppTheme.warning;
          label = '连接中...';
          dot = AnimatedBuilder(
            animation: _controller,
            builder: (context, child) => Container(
              width: 8,
              height: 8,
              decoration: BoxDecoration(
                color: color.withValues(alpha: 0.5 + 0.5 * _controller.value),
                shape: BoxShape.circle,
              ),
            ),
          );
          break;
        default:
          color = Colors.grey;
          label = '离线';
          dot = Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          );
          break;
      }
    }

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        dot,
        const SizedBox(width: 4),
        Text(
          label,
          style: TextStyle(
            fontSize: 12,
            color: Colors.white.withValues(alpha: 0.8),
          ),
        ),
      ],
    );
  }
}

class MessageBubble extends StatelessWidget {
  final ConversationMessage message;
  final bool isMe;
  final bool isConnected;
  final VoidCallback? onEdit;

  const MessageBubble({
    super.key,
    required this.message,
    required this.isMe,
    required this.isConnected,
    this.onEdit,
  });

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: isMe ? Alignment.centerRight : Alignment.centerLeft,
      child: GestureDetector(
        onLongPress: onEdit,
        child: Container(
          constraints: BoxConstraints(
            maxWidth: MediaQuery.of(context).size.width * 0.75,
          ),
          margin: const EdgeInsets.symmetric(vertical: 4),
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: isMe ? AppTheme.primary : Colors.grey[200],
            borderRadius: BorderRadius.circular(16).copyWith(
              bottomRight: isMe
                  ? const Radius.circular(0)
                  : const Radius.circular(16),
              bottomLeft: !isMe
                  ? const Radius.circular(0)
                  : const Radius.circular(16),
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if ((message.imageUrl != null && message.imageUrl!.isNotEmpty) ||
                  (message.imageBase64 != null &&
                      message.imageBase64!.isNotEmpty))
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(8),
                    child:
                        message.imageUrl != null && message.imageUrl!.isNotEmpty
                        ? Image.network(
                            message.imageUrl!,
                            width: 200,
                            fit: BoxFit.cover,
                            errorBuilder: (context, error, stackTrace) {
                              if (message.imageBase64 != null &&
                                  message.imageBase64!.isNotEmpty) {
                                return Image.memory(
                                  base64Decode(message.imageBase64!),
                                  width: 200,
                                  fit: BoxFit.cover,
                                );
                              }
                              return const SizedBox.shrink();
                            },
                          )
                        : Image.memory(
                            base64Decode(message.imageBase64!),
                            width: 200,
                            fit: BoxFit.cover,
                          ),
                  ),
                ),
              if ((message.audioUrl != null && message.audioUrl!.isNotEmpty) ||
                  (message.audioBase64 != null &&
                      message.audioBase64!.isNotEmpty))
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: AudioMessagePlayer(
                    audioUrl: message.audioUrl,
                    audioBase64: message.audioBase64,
                    isMe: isMe,
                  ),
                ),
              Text(
                message.content,
                style: TextStyle(
                  color: isMe ? Colors.white : Colors.black87,
                  fontSize: 16,
                ),
              ),
              if (message.editedAt != null)
                Padding(
                  padding: const EdgeInsets.only(top: 2),
                  child: Text(
                    '（已编辑）',
                    style: TextStyle(
                      fontSize: 10,
                      color: isMe ? Colors.white60 : Colors.black38,
                      fontStyle: FontStyle.italic,
                    ),
                  ),
                ),
              const SizedBox(height: 4),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    _formatTime(message.sentAt),
                    style: TextStyle(
                      fontSize: 10,
                      color: isMe ? Colors.white70 : Colors.black45,
                    ),
                  ),
                  if (isMe && onEdit != null) ...[
                    const SizedBox(width: 4),
                    GestureDetector(
                      onTap: onEdit,
                      child: Text(
                        '编辑',
                        style: TextStyle(
                          fontSize: 10,
                          color: Colors.white60,
                          decoration: TextDecoration.underline,
                        ),
                      ),
                    ),
                  ],
                  if (isMe) ...[const SizedBox(width: 4), _buildStatus()],
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildStatus() {
    if (!isConnected) {
      return const SizedBox.shrink();
    }
    switch (message.status) {
      case 'sending':
        return const Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox(
              width: 8,
              height: 8,
              child: CircularProgressIndicator(
                strokeWidth: 1,
                color: Colors.white54,
              ),
            ),
            SizedBox(width: 2),
            Text('发送中', style: TextStyle(fontSize: 10, color: Colors.white54)),
          ],
        );
      case 'sent':
        return const Text(
          '已发送',
          style: TextStyle(fontSize: 10, color: Colors.white70),
        );
      case 'delivered':
        return const Text(
          '已送达',
          style: TextStyle(fontSize: 10, color: Colors.white70),
        );
      case 'read':
        return const Text(
          '已读',
          style: TextStyle(fontSize: 10, color: AppTheme.success),
        );
      case 'failed':
        return const Text(
          '发送失败',
          style: TextStyle(fontSize: 10, color: Colors.red),
        );
      default:
        return const SizedBox.shrink();
    }
  }

  String _formatTime(DateTime dt) {
    return '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }
}
