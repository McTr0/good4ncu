import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

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
      defects: json['defect_hint'] != null
          ? [json['defect_hint'] as String]
          : (json['defects'] != null
                ? List<String>.from(json['defects'])
                : null),
      ownerId: json['owner_id'],
      ownerUsername: json['owner_username'],
      createdAt: json['created_at'],
    );
  }

  String get conditionLabel => AppTheme.conditionLabel(conditionScore);

  Color get conditionColor => AppTheme.conditionColor(conditionScore);
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

class WatchlistItem {
  final String listingId;
  final String title;
  final String category;
  final String brand;
  final int conditionScore;
  final double suggestedPriceCny;
  final String status;
  final String ownerId;
  final String createdAt;

  const WatchlistItem({
    required this.listingId,
    required this.title,
    required this.category,
    required this.brand,
    required this.conditionScore,
    required this.suggestedPriceCny,
    required this.status,
    required this.ownerId,
    required this.createdAt,
  });

  factory WatchlistItem.fromJson(Map<String, dynamic> json) {
    return WatchlistItem(
      listingId: json['listing_id']?.toString() ?? '',
      title: json['title'] ?? '',
      category: json['category'] ?? '',
      brand: json['brand'] ?? '',
      conditionScore: json['condition_score'] ?? 0,
      suggestedPriceCny: (json['suggested_price_cny'] ?? 0).toDouble(),
      status: json['status'] ?? 'active',
      ownerId: json['owner_id']?.toString() ?? '',
      createdAt: json['created_at'] ?? '',
    );
  }
}

class WatchlistResponse {
  final List<WatchlistItem> items;
  final int total;
  final int limit;
  final int offset;

  const WatchlistResponse({
    required this.items,
    required this.total,
    required this.limit,
    required this.offset,
  });

  factory WatchlistResponse.fromJson(Map<String, dynamic> json) {
    return WatchlistResponse(
      items: (json['items'] as List<dynamic>? ?? [])
          .map((e) => WatchlistItem.fromJson(e as Map<String, dynamic>))
          .toList(),
      total: json['total'] ?? 0,
      limit: json['limit'] ?? 20,
      offset: json['offset'] ?? 0,
    );
  }
}

class AppNotification {
  final String id;
  final String eventType;
  final String title;
  final String body;
  final String? relatedOrderId;
  final String? relatedListingId;
  final bool isRead;
  final String createdAt;

  const AppNotification({
    required this.id,
    required this.eventType,
    required this.title,
    required this.body,
    this.relatedOrderId,
    this.relatedListingId,
    required this.isRead,
    required this.createdAt,
  });

  factory AppNotification.fromJson(Map<String, dynamic> json) {
    return AppNotification(
      id: json['id']?.toString() ?? '',
      eventType: json['event_type'] ?? '',
      title: json['title'] ?? '',
      body: json['body'] ?? '',
      relatedOrderId: json['related_order_id']?.toString(),
      relatedListingId: json['related_listing_id']?.toString(),
      isRead: json['is_read'] ?? false,
      createdAt: json['created_at'] ?? '',
    );
  }

  AppNotification copyWith({
    String? id,
    String? eventType,
    String? title,
    String? body,
    String? relatedOrderId,
    String? relatedListingId,
    bool? isRead,
    String? createdAt,
  }) {
    return AppNotification(
      id: id ?? this.id,
      eventType: eventType ?? this.eventType,
      title: title ?? this.title,
      body: body ?? this.body,
      relatedOrderId: relatedOrderId ?? this.relatedOrderId,
      relatedListingId: relatedListingId ?? this.relatedListingId,
      isRead: isRead ?? this.isRead,
      createdAt: createdAt ?? this.createdAt,
    );
  }
}

class NotificationsResponse {
  final List<AppNotification> items;
  final int total;
  final int unreadCount;
  final int limit;
  final int offset;

  const NotificationsResponse({
    required this.items,
    required this.total,
    required this.unreadCount,
    required this.limit,
    required this.offset,
  });

  factory NotificationsResponse.fromJson(Map<String, dynamic> json) {
    return NotificationsResponse(
      items: (json['items'] as List<dynamic>? ?? [])
          .map((e) => AppNotification.fromJson(e as Map<String, dynamic>))
          .toList(),
      total: json['total'] ?? 0,
      unreadCount: json['unread_count'] ?? 0,
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
  online, // connected + established
  offline, // connected but not established
  pending, // pending
}

/// 会话信息
class Conversation {
  final String id;

  /// The user who initiated the connection request
  final String requesterId;
  final String otherUserId;
  final String otherUsername;
  final String status; // 'connected' | 'pending' | 'established'
  final String? lastMessage;
  final DateTime? lastMessageAt;

  /// 未读消息数
  final int unreadCount;

  /// Whether current user is the receiver of this connection request
  final bool isReceiver;

  Conversation({
    required this.id,
    required this.requesterId,
    required this.otherUserId,
    required this.otherUsername,
    required this.status,
    this.lastMessage,
    this.lastMessageAt,
    this.unreadCount = 0,
    this.isReceiver = false,
  });

  /// Current user can accept/reject this connection (true only for pending incoming requests)
  bool get canRespond => status == 'pending' && isReceiver;

  factory Conversation.fromJson(Map<String, dynamic> json) {
    return Conversation(
      id: json['id']?.toString() ?? '',
      requesterId: json['requester_id']?.toString() ?? '',
      otherUserId: json['other_user_id']?.toString() ?? '',
      otherUsername: json['other_username'] ?? '',
      status: json['status'] ?? 'pending',
      lastMessage: json['last_message'],
      lastMessageAt: json['last_message_at'] != null
          ? DateTime.tryParse(json['last_message_at'].toString())
          : null,
      unreadCount: json['unread_count'] ?? 0,
      isReceiver: json['is_receiver'] ?? false,
    );
  }

  ConnectionStatusType get connectionStatus {
    if (status == 'pending') return ConnectionStatusType.pending;
    // 'connected' and 'established' both mean the connection is active
    if (status == 'connected' || status == 'established') {
      return ConnectionStatusType.online;
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

/// Order summary for list view.
class Order {
  final String id;
  final String listingId;
  final String listingTitle;
  final String buyerId;
  final String sellerId;
  final String buyerUsername;
  final String sellerUsername;
  final double finalPriceCny;
  final String status;
  final String createdAt;
  final String role;

  const Order({
    required this.id,
    required this.listingId,
    required this.listingTitle,
    required this.buyerId,
    required this.sellerId,
    required this.buyerUsername,
    required this.sellerUsername,
    required this.finalPriceCny,
    required this.status,
    required this.createdAt,
    required this.role,
  });

  factory Order.fromJson(Map<String, dynamic> json) {
    return Order(
      id: json['id'] ?? '',
      listingId: json['listing_id'] ?? '',
      listingTitle: json['listing_title'] ?? '',
      buyerId: json['buyer_id'] ?? '',
      sellerId: json['seller_id'] ?? '',
      buyerUsername: json['buyer_username'] ?? '',
      sellerUsername: json['seller_username'] ?? '',
      finalPriceCny: (json['final_price_cny'] ?? 0).toDouble(),
      status: json['status'] ?? 'pending',
      createdAt: json['created_at'] ?? '',
      role: json['role'] ?? 'buyer',
    );
  }

  String get statusLabel {
    switch (status) {
      case 'pending':
        return '待支付';
      case 'paid':
        return '已支付';
      case 'shipped':
        return '已发货';
      case 'completed':
        return '已完成';
      case 'cancelled':
        return '已取消';
      default:
        return status;
    }
  }

  Color get statusColor {
    switch (status) {
      case 'pending':
        return const Color(0xFFF59E0B);
      case 'paid':
        return const Color(0xFF3B82F6);
      case 'shipped':
        return const Color(0xFF8B5CF6);
      case 'completed':
        return const Color(0xFF10B981);
      case 'cancelled':
        return const Color(0xFF6B7280);
      default:
        return Colors.grey;
    }
  }
}

class OrdersResponse {
  final List<Order> items;
  final int total;
  final int limit;
  final int offset;

  const OrdersResponse({
    required this.items,
    required this.total,
    required this.limit,
    required this.offset,
  });

  factory OrdersResponse.fromJson(Map<String, dynamic> json) {
    return OrdersResponse(
      items: (json['items'] as List<dynamic>)
          .map((e) => Order.fromJson(e as Map<String, dynamic>))
          .toList(),
      total: json['total'] ?? 0,
      limit: json['limit'] ?? 20,
      offset: json['offset'] ?? 0,
    );
  }
}

class OrderDetail {
  final String id;
  final String listingId;
  final String listingTitle;
  final String buyerId;
  final String sellerId;
  final String buyerUsername;
  final String sellerUsername;
  final double finalPriceCny;
  final String status;
  final String createdAt;
  final String? paidAt;
  final String? shippedAt;
  final String? completedAt;
  final String? cancelledAt;
  final String? cancellationReason;

  const OrderDetail({
    required this.id,
    required this.listingId,
    required this.listingTitle,
    required this.buyerId,
    required this.sellerId,
    required this.buyerUsername,
    required this.sellerUsername,
    required this.finalPriceCny,
    required this.status,
    required this.createdAt,
    this.paidAt,
    this.shippedAt,
    this.completedAt,
    this.cancelledAt,
    this.cancellationReason,
  });

  factory OrderDetail.fromJson(Map<String, dynamic> json) {
    return OrderDetail(
      id: json['id'] ?? '',
      listingId: json['listing_id'] ?? '',
      listingTitle: json['listing_title'] ?? '',
      buyerId: json['buyer_id'] ?? '',
      sellerId: json['seller_id'] ?? '',
      buyerUsername: json['buyer_username'] ?? '',
      sellerUsername: json['seller_username'] ?? '',
      finalPriceCny: (json['final_price_cny'] ?? 0).toDouble(),
      status: json['status'] ?? 'pending',
      createdAt: json['created_at'] ?? '',
      paidAt: json['paid_at'],
      shippedAt: json['shipped_at'],
      completedAt: json['completed_at'],
      cancelledAt: json['cancelled_at'],
      cancellationReason: json['cancellation_reason'],
    );
  }

  String get statusLabel {
    switch (status) {
      case 'pending':
        return '待支付';
      case 'paid':
        return '已支付';
      case 'shipped':
        return '已发货';
      case 'completed':
        return '已完成';
      case 'cancelled':
        return '已取消';
      default:
        return status;
    }
  }

  Color get statusColor {
    switch (status) {
      case 'pending':
        return const Color(0xFFF59E0B);
      case 'paid':
        return const Color(0xFF3B82F6);
      case 'shipped':
        return const Color(0xFF8B5CF6);
      case 'completed':
        return const Color(0xFF10B981);
      case 'cancelled':
        return const Color(0xFF6B7280);
      default:
        return Colors.grey;
    }
  }

  bool get canPay => status == 'pending';
  bool get canShip => status == 'paid';
  bool get canConfirm => status == 'shipped';
  bool get canCancel => status == 'pending' || status == 'paid';
}
