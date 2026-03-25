use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::mpsc;

pub mod chat;
pub mod hitl_expire;
pub mod notification;
pub mod order;
pub mod product;
pub mod settlement;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BusinessEvent {
    DealReached {
        listing_id: String,
        buyer_id: String,
        seller_id: String,
        final_price: i64,
    },
    OrderPaid {
        order_id: String,
    },
    ChatMessage {
        conversation_id: String,
        listing_id: String,
        sender: String,
        content: String,
        image_data: Option<String>,
        audio_data: Option<String>,
    },
}

pub struct ServiceManager {
    pub product: product::ProductService,
    pub order: order::OrderService,
    pub chat: chat::ChatService,
    pub notification: notification::NotificationService,
    pub settlement: settlement::SettlementService,
    pub event_tx: mpsc::Sender<BusinessEvent>,
}

/// Bounded channel capacity for event bus — provides backpressure when
/// the event loop is overwhelmed, preventing unbounded memory growth.
const EVENT_BUS_CAPACITY: usize = 2048;

impl ServiceManager {
    /// Creates ServiceManager from a single PgPool (serves both relational + vector data).
    #[allow(dead_code)]
    pub fn new(db: PgPool) -> (Self, mpsc::Receiver<BusinessEvent>) {
        let (tx, rx) = mpsc::channel(EVENT_BUS_CAPACITY);

        let manager = Self {
            product: product::ProductService::new(db.clone()),
            order: order::OrderService::new(db.clone()),
            chat: chat::ChatService::new(db.clone()),
            notification: notification::NotificationService::new(db.clone()),
            settlement: settlement::SettlementService::new(db),
            event_tx: tx,
        };

        (manager, rx)
    }

    pub async fn run_event_loop(self, mut rx: mpsc::Receiver<BusinessEvent>) {
        tracing::info!("Business Event Loop started.");
        while let Some(event) = rx.recv().await {
            let order_svc = self.order.clone();
            let product_svc = self.product.clone();
            let settlement_svc = self.settlement.clone();
            let chat_svc = self.chat.clone();
            let notification_svc = self.notification.clone();

            tokio::spawn(async move {
                match event {
                    BusinessEvent::DealReached {
                        listing_id,
                        buyer_id,
                        seller_id,
                        final_price,
                    } => {
                        tracing::info!(listing_id, "DealReached event received");
                        let order_id = match order_svc
                            .create_order(&listing_id, &buyer_id, &seller_id, final_price)
                            .await
                        {
                            Ok(id) => id,
                            Err(e) => {
                                tracing::error!(%e, listing_id, "Failed to create order");
                                return;
                            }
                        };
                        // Note: create_order already does UPDATE inventory SET status='sold'
                        // atomically (within its transaction), so mark_as_sold below is
                        // idempotent and will be a no-op for the winning buyer.
                        if let Err(e) = product_svc.mark_as_sold(&listing_id).await {
                            tracing::error!(%e, listing_id, "Failed to mark listing as sold");
                        }
                        // Notify seller that their item was purchased
                        let _ = notification_svc
                            .create(
                                &seller_id,
                                "deal_reached",
                                "订单已创建",
                                &format!("商品已被购买，成交价 ¥{:.2}", final_price as f64 / 100.0),
                                Some(&order_id),
                                Some(&listing_id),
                            )
                            .await;
                    }
                    BusinessEvent::OrderPaid { order_id } => {
                        tracing::info!(order_id, "OrderPaid event received");

                        // First get order details — needed for notifications regardless of
                        // settlement outcome. Must fetch before spawning to get seller_id.
                        let order_details = order_svc.get_order(&order_id).await;
                        let (seller_id, listing_id) = match order_details {
                            Ok(Some((sid, lid))) => (sid, lid),
                            Ok(None) => {
                                tracing::error!(order_id, "Order not found");
                                return;
                            }
                            Err(e) => {
                                tracing::error!(%e, order_id, "Failed to get order");
                                return;
                            }
                        };

                        // Spawn settlement as independent task — does not block event loop.
                        // Settlement failure does NOT block order status update + notification.
                        let order_id_clone = order_id.clone();
                        let settlement_svc_clone = settlement_svc.clone();
                        let seller_id_clone = seller_id.clone();
                        let notification_svc_clone = notification_svc.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                settlement_svc_clone.finalize_payment(&order_id_clone).await
                            {
                                tracing::error!(%e, order_id_clone, "Settlement failed");
                                // Notify seller of settlement failure
                                let _ = notification_svc_clone
                                    .create(
                                        &seller_id_clone,
                                        "settlement_failed",
                                        "结算失败",
                                        &format!("订单结算失败: {}", e),
                                        Some(&order_id_clone),
                                        None,
                                    )
                                    .await;
                            }
                        });

                        // Order status update and payment notification run in main event loop
                        if let Err(e) = order_svc.update_order_status(&order_id, "paid").await {
                            tracing::error!(%e, order_id, "Failed to update order status");
                        }
                        // Notify seller that payment was received
                        let _ = notification_svc
                            .create(
                                &seller_id,
                                "order_paid",
                                "款项已到账",
                                "买家已付款，请尽快发货",
                                Some(&order_id),
                                Some(&listing_id),
                            )
                            .await;
                    }
                    BusinessEvent::ChatMessage {
                        conversation_id,
                        listing_id,
                        sender,
                        content,
                        image_data,
                        audio_data,
                    } => {
                        // receiver is None here — the event-based path is a fallback for
                        // message logging and doesn't carry receiver context. The primary
                        // path (api/mod.rs handle_chat) also passes None since it has no
                        // listing context. Proper receiver population requires a follow-up
                        // to thread listing context through the event.
                        if let Err(e) = chat_svc
                            .log_message(
                                &conversation_id,
                                &listing_id,
                                &sender,
                                None,
                                false, // user message, not agent
                                &content,
                                image_data.as_deref(),
                                audio_data.as_deref(),
                            )
                            .await
                        {
                            tracing::error!(%e, conversation_id, "Failed to log chat message");
                        }
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_business_event_deal_reached_serialization() {
        let event = BusinessEvent::DealReached {
            listing_id: "listing-123".to_string(),
            buyer_id: "buyer-456".to_string(),
            seller_id: "seller-789".to_string(),
            final_price: 4999,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("DealReached"));
        assert!(json.contains("listing-123"));
        assert!(json.contains("buyer-456"));
        assert!(json.contains("seller-789"));
        assert!(json.contains("4999"));
    }

    #[test]
    fn test_business_event_order_paid_serialization() {
        let event = BusinessEvent::OrderPaid {
            order_id: "order-abc".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("OrderPaid"));
        assert!(json.contains("order-abc"));
    }

    #[test]
    fn test_business_event_chat_message_serialization() {
        let event = BusinessEvent::ChatMessage {
            conversation_id: "conv-1".to_string(),
            listing_id: "listing-1".to_string(),
            sender: "user-1".to_string(),
            content: "Hello!".to_string(),
            image_data: Some("base64image".to_string()),
            audio_data: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ChatMessage"));
        assert!(json.contains("conv-1"));
        assert!(json.contains("Hello!"));
        assert!(json.contains("base64image"));
    }

    #[test]
    fn test_business_event_chat_message_without_optional() {
        let event = BusinessEvent::ChatMessage {
            conversation_id: "conv-2".to_string(),
            listing_id: "listing-2".to_string(),
            sender: "user-2".to_string(),
            content: "Hi".to_string(),
            image_data: None,
            audio_data: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ChatMessage"));
        assert!(json.contains("conv-2"));
    }

    #[test]
    fn test_business_event_deserialization() {
        let json = r#"{
            "DealReached": {
                "listing_id": "listing-x",
                "buyer_id": "buyer-y",
                "seller_id": "seller-z",
                "final_price": 2999
            }
        }"#;
        let event: BusinessEvent = serde_json::from_str(json).unwrap();
        match event {
            BusinessEvent::DealReached {
                listing_id,
                buyer_id,
                seller_id,
                final_price,
            } => {
                assert_eq!(listing_id, "listing-x");
                assert_eq!(buyer_id, "buyer-y");
                assert_eq!(seller_id, "seller-z");
                assert_eq!(final_price, 2999);
            }
            _ => panic!("Expected DealReached variant"),
        }
    }

    #[test]
    fn test_event_bus_capacity_constant() {
        // Verify the constant is a reasonable size for backpressure
        assert!(EVENT_BUS_CAPACITY >= 100);
        assert!(EVENT_BUS_CAPACITY <= 100_000);
    }
}
