use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::mpsc;

pub mod chat;
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

            tokio::spawn(async move {
                match event {
                    BusinessEvent::DealReached {
                        listing_id,
                        buyer_id,
                        seller_id,
                        final_price,
                    } => {
                        tracing::info!(listing_id, "DealReached event received");
                        if let Err(e) = order_svc
                            .create_order(&listing_id, &buyer_id, &seller_id, final_price)
                            .await
                        {
                            tracing::error!(%e, listing_id, "Failed to create order");
                        }
                        if let Err(e) = product_svc.mark_as_sold(&listing_id).await {
                            tracing::error!(%e, listing_id, "Failed to mark listing as sold");
                        }
                    }
                    BusinessEvent::OrderPaid { order_id } => {
                        tracing::info!(order_id, "OrderPaid event received");
                        if let Err(e) = settlement_svc.finalize_payment(&order_id).await {
                            tracing::error!(%e, order_id, "Failed to finalize payment");
                        }
                        if let Err(e) = order_svc.update_order_status(&order_id, "completed").await
                        {
                            tracing::error!(%e, order_id, "Failed to update order status");
                        }
                    }
                    BusinessEvent::ChatMessage {
                        conversation_id,
                        listing_id,
                        sender,
                        content,
                        image_data,
                        audio_data,
                    } => {
                        if let Err(e) = chat_svc
                            .log_message(
                                &conversation_id,
                                &listing_id,
                                &sender,
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
