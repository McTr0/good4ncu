import 'package:flutter/material.dart';

class Listing {
  final String id;
  final String title;
  final String category;
  final String brand;
  final int conditionScore;
  final double suggestedPriceCny;
  final String? description;
  final String status;
  final String? thumbnailHint;
  final List<String>? defects;
  final String? ownerId;
  final String? ownerUsername;
  final String? createdAt;

  Listing({
    required this.id,
    required this.title,
    required this.category,
    required this.brand,
    required this.conditionScore,
    required this.suggestedPriceCny,
    this.description,
    required this.status,
    this.thumbnailHint,
    this.defects,
    this.ownerId,
    this.ownerUsername,
    this.createdAt,
  });

  factory Listing.fromJson(Map<String, dynamic> json) {
    return Listing(
      id: json['id'] ?? '',
      title: json['title'] ?? '',
      category: json['category'] ?? '',
      brand: json['brand'] ?? '',
      conditionScore: json['condition_score'] ?? 0,
      suggestedPriceCny: (json['suggested_price_cny'] ?? 0).toDouble(),
      description: json['description'],
      status: json['status'] ?? 'active',
      thumbnailHint: json['thumbnail_hint'],
      defects: json['defects'] != null
          ? List<String>.from(json['defects'])
          : null,
      ownerId: json['owner_id'],
      ownerUsername: json['owner_username'],
      createdAt: json['created_at'],
    );
  }

  String get conditionLabel {
    if (conditionScore >= 9) return '几乎全新';
    if (conditionScore >= 7) return '较好';
    if (conditionScore >= 5) return '一般';
    return '较差';
  }

  Color get conditionColor {
    if (conditionScore >= 9) return const Color(0xFF10B981);
    if (conditionScore >= 7) return const Color(0xFF3B82F6);
    if (conditionScore >= 5) return const Color(0xFFF59E0B);
    return const Color(0xFFEF4444);
  }
}

class ListingsResponse {
  final List<Listing> items;
  final int total;
  final int limit;
  final int offset;

  ListingsResponse({
    required this.items,
    required this.total,
    required this.limit,
    required this.offset,
  });

  factory ListingsResponse.fromJson(Map<String, dynamic> json) {
    return ListingsResponse(
      items: (json['items'] as List<dynamic>)
          .map((e) => Listing.fromJson(e as Map<String, dynamic>))
          .toList(),
      total: json['total'] ?? 0,
      limit: json['limit'] ?? 20,
      offset: json['offset'] ?? 0,
    );
  }
}

class ChatMessage {
  final String sender;
  final String content;
  final String? imageBase64;
  final String? audioBase64;
  final DateTime timestamp;
  /// True while the SSE stream is still delivering tokens (typing indicator).
  final bool isPartial;

  ChatMessage({
    required this.sender,
    required this.content,
    this.imageBase64,
    this.audioBase64,
    required this.timestamp,
    this.isPartial = false,
  });

  ChatMessage copyWith({
    String? sender,
    String? content,
    String? imageBase64,
    String? audioBase64,
    DateTime? timestamp,
    bool? isPartial,
  }) {
    return ChatMessage(
      sender: sender ?? this.sender,
      content: content ?? this.content,
      imageBase64: imageBase64 ?? this.imageBase64,
      audioBase64: audioBase64 ?? this.audioBase64,
      timestamp: timestamp ?? this.timestamp,
      isPartial: isPartial ?? this.isPartial,
    );
  }

  Map<String, dynamic> toJson() => {
        'message': content,
        'image': imageBase64,
        'audio': audioBase64,
      };
}

/// 连接状态类型
enum ConnectionStatusType {
  online,   // connected + established
  offline,  // connected but not established
  pending,  // pending
}

/// 会话信息
class Conversation {
  final String id;
  final String otherUserId;
  final String otherUsername;
  final String status; // 'connected' | 'pending' | 'established'
  final String? lastMessage;
  final DateTime? lastMessageAt;
  /// 未读消息数
  final int unreadCount;

  Conversation({
    required this.id,
    required this.otherUserId,
    required this.otherUsername,
    required this.status,
    this.lastMessage,
    this.lastMessageAt,
    this.unreadCount = 0,
  });

  factory Conversation.fromJson(Map<String, dynamic> json) {
    return Conversation(
      id: json['id']?.toString() ?? '',
      otherUserId: json['other_user_id']?.toString() ?? '',
      otherUsername: json['other_username'] ?? '',
      status: json['status'] ?? 'pending',
      lastMessage: json['last_message'],
      lastMessageAt: json['last_message_at'] != null
          ? DateTime.tryParse(json['last_message_at'].toString())
          : null,
      unreadCount: json['unread_count'] ?? 0,
    );
  }

  ConnectionStatusType get connectionStatus {
    if (status == 'pending') return ConnectionStatusType.pending;
    if (status == 'connected' || status == 'established') {
      if (status == 'established') return ConnectionStatusType.online;
      return ConnectionStatusType.offline;
    }
    return ConnectionStatusType.offline;
  }
}

/// 私聊消息
class ConversationMessage {
  final String id;
  final String conversationId;
  final String senderId;
  final String content;
  final String? imageBase64;
  final String? audioBase64;
  final DateTime sentAt;
  final DateTime? readAt;
  /// 消息状态: sending | sent | delivered | read | failed
  final String status;
  /// 已编辑时间
  final DateTime? editedAt;

  ConversationMessage({
    required this.id,
    required this.conversationId,
    required this.senderId,
    required this.content,
    this.imageBase64,
    this.audioBase64,
    required this.sentAt,
    this.readAt,
    this.status = 'sent',
    this.editedAt,
  });

  factory ConversationMessage.fromJson(Map<String, dynamic> json) {
    return ConversationMessage(
      id: json['id']?.toString() ?? '',
      conversationId: json['conversation_id']?.toString() ?? '',
      senderId: json['sender']?.toString() ?? '',
      content: json['content'] ?? '',
      imageBase64: json['image_base64'] ?? json['image_data'],
      audioBase64: json['audio_base64'] ?? json['audio_data'],
      sentAt: json['sent_at'] != null
          ? DateTime.parse(json['sent_at'].toString())
          : json['timestamp'] != null
              ? DateTime.parse(json['timestamp'].toString())
              : DateTime.now(),
      readAt: json['read_at'] != null
          ? DateTime.tryParse(json['read_at'].toString())
          : null,
      status: json['status'] ?? 'sent',
      editedAt: json['edited_at'] != null
          ? DateTime.tryParse(json['edited_at'].toString())
          : null,
    );
  }

  /// 消息是否已读（有连接且已读）
  bool get isRead => readAt != null;

  /// 消息是否由指定用户发送
  bool isFrom(String userId) => senderId == userId;

  /// 是否可编辑（发送后15分钟内）
  bool get canEdit {
    if (editedAt != null) return false;
    final diff = DateTime.now().difference(sentAt);
    return diff.inMinutes < 15;
  }

  ConversationMessage copyWith({
    String? id,
    String? conversationId,
    String? senderId,
    String? content,
    String? imageBase64,
    String? audioBase64,
    DateTime? sentAt,
    DateTime? readAt,
    String? status,
    DateTime? editedAt,
  }) {
    return ConversationMessage(
      id: id ?? this.id,
      conversationId: conversationId ?? this.conversationId,
      senderId: senderId ?? this.senderId,
      content: content ?? this.content,
      imageBase64: imageBase64 ?? this.imageBase64,
      audioBase64: audioBase64 ?? this.audioBase64,
      sentAt: sentAt ?? this.sentAt,
      readAt: readAt ?? this.readAt,
      status: status ?? this.status,
      editedAt: editedAt ?? this.editedAt,
    );
  }
}

class HitlRequest {
  final String id;
  final String listingId;
  final String buyerId;
  final String sellerId;
  final double proposedPrice;
  final String reason;
  final String status; // pending | countered | approved | rejected | expired
  final double? counterPrice;
  final String createdAt;
  final String? expiresAt;

  HitlRequest({
    required this.id,
    required this.listingId,
    required this.buyerId,
    required this.sellerId,
    required this.proposedPrice,
    required this.reason,
    required this.status,
    this.counterPrice,
    required this.createdAt,
    this.expiresAt,
  });

  factory HitlRequest.fromJson(Map<String, dynamic> json) {
    return HitlRequest(
      id: json['id'] ?? '',
      listingId: json['listing_id'] ?? '',
      buyerId: json['buyer_id'] ?? '',
      sellerId: json['seller_id'] ?? '',
      proposedPrice: (json['proposed_price'] ?? 0).toDouble(),
      reason: json['reason'] ?? '',
      status: json['status'] ?? 'pending',
      counterPrice: json['counter_price']?.toDouble(),
      createdAt: json['created_at'] ?? '',
      expiresAt: json['expires_at'],
    );
  }

  bool get isPending => status == 'pending';
  bool get isCountered => status == 'countered';
  bool get isExpired => status == 'expired';
  bool get isFinal => status == 'approved' || status == 'rejected';
}
