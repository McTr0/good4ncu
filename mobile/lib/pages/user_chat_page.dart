import 'package:image_picker/image_picker.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/models.dart';
import '../providers/chat_notifier.dart';
import '../services/chat_service.dart';
import '../services/upload_service.dart';
import '../services/user_service.dart';
import '../services/ws_service.dart';
import '../theme/app_theme.dart';
import 'user_chat_composer_controller.dart';
import 'user_chat_components.dart';
import 'user_chat_media_sender.dart';
import 'user_chat_session_controller.dart';

export 'user_chat_components.dart' show ConnectionIndicator, MessageBubble;

/// 私聊页面
class UserChatPage extends StatefulWidget {
  final String conversationId;
  final String otherUserId;
  final String otherUsername;
  final ChatService? chatService;
  final UserService? userService;
  final UploadService? uploadService;
  final ChatNotifier? chatNotifier;
  final UserChatMediaSender? mediaSender;
  final UserChatSessionController? sessionController;
  final UserChatComposerController? composerController;

  const UserChatPage({
    super.key,
    required this.conversationId,
    required this.otherUserId,
    required this.otherUsername,
    this.chatService,
    this.userService,
    this.uploadService,
    this.chatNotifier,
    this.mediaSender,
    this.sessionController,
    this.composerController,
  }) : assert(
         composerController == null || chatNotifier != null,
         'Injecting composerController also requires injecting chatNotifier.',
       );

  @override
  State<UserChatPage> createState() => _UserChatPageState();
}

class _UserChatPageState extends State<UserChatPage> {
  late final ChatService _chatService;
  late final UserService _userService;
  late final UserChatMediaSender _mediaSender;
  late final UserChatSessionController _sessionController;
  late final bool _ownsSessionController;
  late final bool _ownsChatNotifier;
  late final UserChatComposerController _composerController;
  late final bool _ownsComposerController;
  late final ChatNotifier _chatNotifier;
  late final void Function() _removeChatListener;
  final ImagePicker _imagePicker = ImagePicker();
  final ScrollController _scrollController = ScrollController();
  ChatViewState _chatState = const ChatViewInitial();

  ChatViewData? get _chatData =>
      _chatState is ChatViewData ? _chatState as ChatViewData : null;

  ChatViewError? get _chatError =>
      _chatState is ChatViewError ? _chatState as ChatViewError : null;

  List<ConversationMessage> get _messages =>
      _chatData?.messages ?? _chatError?.messages ?? const [];

  String? get _currentUserId => _chatData?.currentUserId;

  String? get _editingMessageId => _chatData?.editingMessageId;

  bool get _isLoading =>
      _chatState is ChatViewInitial || _chatState is ChatViewLoading;

  bool get _isSending => _chatData?.isSending ?? false;

  String? get _error => _chatError?.message;

  String? get _connectionStatus => _chatData?.connectionStatus;

  bool get _isOtherTyping => _chatData?.isOtherTyping ?? false;

  bool get _isRecording => _sessionController.isRecording;

  int get _recordingSeconds => _sessionController.recordingSeconds;

  @override
  void initState() {
    super.initState();
    _chatService = widget.chatService ?? context.read<ChatService>();
    _userService = widget.userService ?? context.read<UserService>();
    final uploadService = widget.uploadService ?? context.read<UploadService>();
    _mediaSender =
        widget.mediaSender ?? UserChatMediaSender(uploadService: uploadService);
    _ownsSessionController = widget.sessionController == null;
    _sessionController =
        widget.sessionController ??
        UserChatSessionController(mediaSender: _mediaSender);
    _ownsChatNotifier = widget.chatNotifier == null;
    _chatNotifier =
        widget.chatNotifier ??
        ChatNotifier(
          conversationId: widget.conversationId,
          chatService: _chatService,
          userService: _userService,
        );
    _ownsComposerController = widget.composerController == null;
    _composerController =
        widget.composerController ??
        UserChatComposerController(chatNotifier: _chatNotifier);
    _removeChatListener = _chatNotifier.addListener(
      _handleChatStateChange,
      fireImmediately: true,
    );
    _sessionController.addListener(_handleSessionStateChange);
    _chatNotifier.hydrateConnectionStatus();
    _sessionController.connectWs(_handleWsNotification);
  }

  @override
  void dispose() {
    _removeChatListener();
    _sessionController.removeListener(_handleSessionStateChange);
    if (_ownsSessionController) {
      _sessionController.dispose();
    }
    if (_ownsComposerController) {
      _composerController.dispose();
    }
    if (_ownsChatNotifier) {
      _chatNotifier.dispose();
    }
    _scrollController.dispose();
    super.dispose();
  }

  void _handleChatStateChange(ChatViewState state) {
    final previousCount = _messages.length;
    if (!mounted) return;
    setState(() {
      _chatState = state;
    });
    final nextCount = _messages.length;
    if (nextCount != previousCount && nextCount > 0) {
      _scrollToBottom();
    }
  }

  void _handleSessionStateChange() {
    if (!mounted) return;
    setState(() {});
  }

  void _handleWsNotification(WsNotification notif) {
    if (!mounted) return;

    switch (notif.eventType) {
      case 'connection_established':
        if (notif.connectionId == widget.conversationId) {
          _chatNotifier.handleWsNotification('connection_established');
          _showSnackBar('连接已建立');
        }
        break;

      case 'connection_rejected':
        if (notif.connectionId == widget.conversationId) {
          _chatNotifier.handleWsNotification('connection_rejected');
          _showSnackBar('连接已被对方拒绝');
        }
        break;

      case 'new_message':
        _chatNotifier.handleWsNotification(
          notif.eventType,
          messageId: notif.messageId,
          conversationId: notif.conversationId,
          typingUserId: notif.typingUserId,
        );
        break;

      case 'message_read':
        _chatNotifier.handleWsNotification(notif.eventType);
        break;

      case 'typing':
        _chatNotifier.handleWsNotification(
          notif.eventType,
          conversationId: notif.conversationId,
          typingUserId: notif.typingUserId,
        );
        break;

      case 'connection_request':
        _showConnectionRequestDialog(notif);
        break;
    }
  }

  void _showConnectionRequestDialog(WsNotification notif) {
    showUserChatConnectionRequestDialog(
      context: context,
      notification: notif,
      onAccept: _acceptConnection,
      onReject: _rejectConnection,
    );
  }

  Future<void> _acceptConnection(String connectionId) async {
    try {
      await _chatNotifier.acceptConnection(connectionId);
      _showSnackBar('已接受连接');
    } catch (e) {
      _showSnackBar('接受失败: $e');
    }
  }

  Future<void> _rejectConnection(String connectionId) async {
    try {
      await _chatNotifier.rejectConnection(connectionId);
      _showSnackBar('已拒绝连接');
    } catch (e) {
      _showSnackBar('拒绝失败: $e');
    }
  }

  /// 发送typing indicator
  void _sendTypingIndicator() {
    _composerController.sendTypingIndicator();
  }

  /// 开始编辑消息
  void _startEditMessage(ConversationMessage msg) {
    _composerController.startEditMessage(msg);
  }

  /// 取消编辑
  void _cancelEdit() {
    _composerController.cancelEdit();
  }

  /// 确认编辑
  Future<void> _confirmEdit() async {
    try {
      final message = await _composerController.confirmEdit();
      if (!mounted || message == null) return;
      _showSnackBar(message);
    } on UserChatComposerException catch (e) {
      if (!mounted) return;
      _showSnackBar(e.message);
    }
  }

  Future<void> _sendMessage() async {
    // 如果正在编辑，先确认编辑
    if (_editingMessageId != null) {
      await _confirmEdit();
      return;
    }

    try {
      await _composerController.sendMessage();
    } on UserChatComposerException catch (e) {
      _showSnackBar(e.message);
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

    try {
      await _mediaSender.sendPickedImage(
        picked,
        sendMessage:
            ({
              required String content,
              String? imageBase64,
              String? audioBase64,
              String? imageUrl,
              String? audioUrl,
            }) => _chatNotifier.sendMessage(
              content: content,
              imageBase64: imageBase64,
              audioBase64: audioBase64,
              imageUrl: imageUrl,
              audioUrl: audioUrl,
            ),
      );
    } on UserChatMediaSendException catch (e) {
      _showSnackBar(e.message);
    } catch (e) {
      _showSnackBar('发送失败: $e');
    }
  }

  /// 切换录音状态
  Future<void> _toggleRecording() async {
    await _sessionController.toggleRecording(
      canSendMedia: () => _connectionStatus == 'connected',
      sendMessage:
          ({
            required String content,
            String? imageBase64,
            String? audioBase64,
            String? imageUrl,
            String? audioUrl,
          }) => _chatNotifier.sendMessage(
            content: content,
            imageBase64: imageBase64,
            audioBase64: audioBase64,
            imageUrl: imageUrl,
            audioUrl: audioUrl,
          ),
      onError: _showSnackBar,
    );
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
            UserChatTypingBanner(username: widget.otherUsername),
          Expanded(
            child: UserChatMessageList(
              isLoading: _isLoading,
              error: _error,
              messages: _messages,
              currentUserId: _currentUserId,
              connectionStatus: _connectionStatus,
              scrollController: _scrollController,
              onRetry: _chatNotifier.loadMessages,
              onEditMessage: _startEditMessage,
            ),
          ),
          UserChatInputArea(
            connectionStatus: _connectionStatus,
            isRecording: _isRecording,
            recordingSeconds: _recordingSeconds,
            isSending: _isSending,
            isEditing: _editingMessageId != null,
            textController: _composerController.textController,
            onPickImage: _pickAndSendImage,
            onToggleRecording: _toggleRecording,
            onCancelEdit: _cancelEdit,
            onChanged: (_) => _sendTypingIndicator(),
            onSubmitted: (_) => _sendMessage(),
            onSend: _editingMessageId != null ? _confirmEdit : _sendMessage,
          ),
        ],
      ),
    );
  }
}
