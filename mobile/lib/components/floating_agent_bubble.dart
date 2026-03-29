import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/agent_chat_notifier.dart';
import '../theme/app_theme.dart';
import 'agent_chat_panel.dart';

/// Floating agent chat bubble — draggable FAB that opens the chat panel overlay.
/// Only visible to logged-in users.
class FloatingAgentBubble extends StatefulWidget {
  final bool isLoggedIn;

  const FloatingAgentBubble({super.key, required this.isLoggedIn});

  @override
  State<FloatingAgentBubble> createState() => _FloatingAgentBubbleState();
}

class _FloatingAgentBubbleState extends State<FloatingAgentBubble> {
  OverlayEntry? _overlayEntry;
  AgentChatNotifier? _notifier;
  final GlobalKey _fabKey = GlobalKey();
  Offset _position = const Offset(16, 16);

  @override
  void didUpdateWidget(FloatingAgentBubble oldWidget) {
    super.didUpdateWidget(oldWidget);
    // If user logs out while overlay is open, close it.
    if (oldWidget.isLoggedIn && !widget.isLoggedIn) {
      _removeOverlay();
    }
  }

  @override
  void dispose() {
    _removeOverlay();
    super.dispose();
  }

  void _showOverlay(BuildContext context, AgentChatNotifier notifier) {
    _removeOverlay();
    _notifier = notifier;

    final overlay = Overlay.of(context);
    final renderBox = _fabKey.currentContext?.findRenderObject() as RenderBox?;
    final globalPosition = renderBox?.localToGlobal(Offset.zero) ?? Offset.zero;

    // Compute overlay position: above the FAB, right-aligned to screen.
    final screenWidth = MediaQuery.of(context).size.width;
    final overlayLeft = (screenWidth - 360 - 16).clamp(
      8.0,
      (screenWidth - 368.0).clamp(8.0, double.infinity),
    );
    final overlayBottom =
        MediaQuery.of(context).size.height - globalPosition.dy + 8;

    _overlayEntry = OverlayEntry(
      builder: (context) => Positioned(
        left: overlayLeft,
        bottom: overlayBottom.clamp(
          80.0,
          (MediaQuery.of(context).size.height - 600).clamp(
            80.0,
            double.infinity,
          ),
        ),
        child: Material(
          color: Colors.transparent,
          child: ChangeNotifierProvider<AgentChatNotifier>.value(
            value: notifier,
            child: AgentChatPanel(onClose: () => _removeOverlay()),
          ),
        ),
      ),
    );

    overlay.insert(_overlayEntry!);
  }

  void _removeOverlay() {
    _notifier?.closePanel();
    _overlayEntry?.remove();
    _overlayEntry = null;
    _notifier = null;
  }

  @override
  Widget build(BuildContext context) {
    if (!widget.isLoggedIn) {
      return const SizedBox.shrink();
    }

    return Positioned(
      right: _position.dx,
      bottom: _position.dy,
      child: GestureDetector(
        onPanUpdate: (details) {
          final screenWidth = MediaQuery.of(context).size.width;
          final screenHeight = MediaQuery.of(context).size.height;

          setState(() {
            _position = Offset(
              (_position.dx - details.delta.dx).clamp(8, screenWidth - 64),
              (_position.dy - details.delta.dy).clamp(8, screenHeight - 64),
            );
          });
        },
        child: FloatingActionButton(
          key: _fabKey,
          heroTag: 'agent_chat_fab',
          onPressed: () async {
            if (_overlayEntry != null) {
              _removeOverlay();
            } else {
              final notifier = context.read<AgentChatNotifier>();
              _showOverlay(context, notifier);
              await notifier.loadHistory();
              if (_overlayEntry != null && !notifier.hasMessages) {
                await notifier.requestGreeting();
              }
            }
          },
          backgroundColor: AppTheme.primary,
          child: const Icon(Icons.smart_toy_outlined, color: Colors.white),
        ),
      ),
    );
  }
}
