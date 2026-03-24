#![allow(dead_code)]
use crate::llm::{LlmProvider, NegotiateAgent};
use crate::services::BusinessEvent;
use crate::utils::{cents_to_yuan, yuan_to_cents};
use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Human approval request stored in database for async resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitlRequest {
    pub id: String,
    pub proposed_price: i64,
    pub reason: String,
    pub status: String, // "pending", "approved", "rejected", "countered"
    pub counter_price: Option<i64>,
}

/// Result of a human approval request
#[derive(Debug, Clone)]
pub enum HitlResult {
    Approved,
    Rejected,
    Countered(i64),
}

/// Shared state for HITL tool - in production this would use a database.
/// For CLI demo, this uses in-memory channel-based approach.
#[derive(Clone)]
pub struct HitlChannel {
    /// For CLI mode: sender to request approval and wait
    pub cli_tx: Option<mpsc::Sender<(HitlRequest, tokio::sync::oneshot::Sender<HitlResult>)>>,
}

impl HitlChannel {
    /// Create a channel-based HITL for CLI mode
    pub fn new_cli() -> (
        Self,
        mpsc::Receiver<(HitlRequest, tokio::sync::oneshot::Sender<HitlResult>)>,
    ) {
        let (tx, rx) = mpsc::channel(1);
        (Self { cli_tx: Some(tx) }, rx)
    }

    /// Create a no-op HITL for testing or when HITL is disabled
    #[allow(dead_code)]
    pub fn new_disabled() -> Self {
        Self { cli_tx: None }
    }

    /// Request human approval - blocks until response received
    pub async fn request_approval(
        &self,
        request: HitlRequest,
    ) -> Result<HitlResult, HumanInteractionError> {
        if let Some(ref tx) = self.cli_tx {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            tx.send((request, resp_tx))
                .await
                .map_err(|e| HumanInteractionError(format!("Channel closed: {}", e)))?;
            resp_rx
                .await
                .map_err(|e| HumanInteractionError(format!("Approval request cancelled: {}", e)))
        } else {
            // No CLI channel configured - auto-reject in production
            Err(HumanInteractionError(
                "HITL not configured - negotiation not available".to_string(),
            ))
        }
    }
}

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct HumanApprovalArgs {
    pub proposed_price: i64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HumanDecision {
    pub action: String,
    pub counter_price: Option<i64>,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Human interaction error: {0}")]
pub struct HumanInteractionError(String);

/// Human approval tool - works with async channel for web context.
/// For CLI demo, use `run_cli_negotiation` instead.
pub struct HumanApprovalTool {
    pub hitl: HitlChannel,
}

impl HumanApprovalTool {
    pub fn new(hitl: HitlChannel) -> Self {
        Self { hitl }
    }
}

impl Tool for HumanApprovalTool {
    const NAME: &'static str = "ask_human_for_approval";
    type Error = HumanInteractionError;
    type Args = HumanApprovalArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "Ask the human user (the owner) to approve, counter, or reject a negotiation offer."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "proposed_price": {
                        "type": "number",
                        "description": "The current proposed price in CNY (in cents)"
                    },
                    "reason": {
                        "type": "string",
                        "description": "A short summary of why the agent is asking for approval"
                    }
                },
                "required": ["proposed_price", "reason"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let request = HitlRequest {
            id: uuid::Uuid::new_v4().to_string(),
            proposed_price: args.proposed_price,
            reason: args.reason,
            status: "pending".to_string(),
            counter_price: None,
        };

        let result = self.hitl.request_approval(request).await?;

        match result {
            HitlResult::Approved => serde_json::to_string(&HumanDecision {
                action: "approve".to_string(),
                counter_price: None,
                message: "Human approved the deal.".to_string(),
            })
            .map_err(|e| HumanInteractionError(format!("JSON error: {}", e))),
            HitlResult::Rejected => serde_json::to_string(&HumanDecision {
                action: "reject".to_string(),
                counter_price: None,
                message: "Human rejected the offer.".to_string(),
            })
            .map_err(|e| HumanInteractionError(format!("JSON error: {}", e))),
            HitlResult::Countered(price) => serde_json::to_string(&HumanDecision {
                action: "counter".to_string(),
                counter_price: Some(price),
                message: format!("Human countered with {} CNY.", cents_to_yuan(price)),
            })
            .map_err(|e| HumanInteractionError(format!("JSON error: {}", e))),
        }
    }
}

/// CLI handler for HITL approval requests
pub async fn run_cli_hitl_handler(
    mut rx: mpsc::Receiver<(HitlRequest, tokio::sync::oneshot::Sender<HitlResult>)>,
) {
    use inquire::{Select, Text};

    while let Some((request, response_tx)) = rx.recv().await {
        println!("\n🔔 [HITL] AGENT IS ASKING FOR YOUR GUIDANCE:");
        println!(
            "Proposed Price: {} CNY",
            cents_to_yuan(request.proposed_price)
        );
        println!("Reason: {}", request.reason);

        let options = vec![
            "Approve (Close deal)",
            "Counter-offer",
            "Reject (Walk away)",
        ];

        match Select::new("Your decision:", options).prompt() {
            Ok(selection) => {
                let result = match selection {
                    "Approve (Close deal)" => HitlResult::Approved,
                    "Counter-offer" => {
                        let price_str = Text::new("Enter counter-offer price (CNY):").prompt();
                        match price_str {
                            Ok(p) => match p.parse::<f64>() {
                                Ok(price) => HitlResult::Countered(yuan_to_cents(price)),
                                Err(_) => HitlResult::Rejected,
                            },
                            Err(_) => HitlResult::Rejected,
                        }
                    }
                    _ => HitlResult::Rejected,
                };
                let _ = response_tx.send(result);
            }
            Err(_) => {
                let _ = response_tx.send(HitlResult::Rejected);
            }
        }
    }
}

#[allow(dead_code)]
pub async fn run_auto_negotiation(
    provider: Arc<dyn LlmProvider>,
    event_tx: mpsc::Sender<BusinessEvent>,
) -> Result<()> {
    tracing::info!("Starting auto-negotiation demo with HITL");

    let listing_id = "demo-herman-miller-chair".to_string();
    let conversation_id = format!("negotiate:{}", listing_id);

    // Create HITL channel for CLI mode
    let (_hitl_channel, hitl_rx) = HitlChannel::new_cli();

    // Spawn CLI HITL handler
    let _hitl_handle = tokio::spawn(run_cli_hitl_handler(hitl_rx));

    // Create seller and buyer agents via provider
    let seller_agent: Box<dyn NegotiateAgent> = provider
        .clone()
        .create_negotiate_agent()
        .await?;

    let buyer_agent: Box<dyn NegotiateAgent> = provider
        .create_negotiate_agent()
        .await?;

    let current_message =
        "Hi, is this chair still available? I'm interested and would like to offer 1700 CNY."
            .to_string();

    tracing::info!(message = %current_message, sender = "Buyer-Bot", "Sending opening message");

    let _ = event_tx
        .send(BusinessEvent::ChatMessage {
            conversation_id: conversation_id.clone(),
            listing_id: listing_id.clone(),
            sender: "Buyer-Bot".to_string(),
            content: current_message.clone(),
            image_data: None,
            audio_data: None,
        })
        .await;

    let mut history = vec![format!("Buyer: {}", current_message)];

    for round in 1..=6 {
        tracing::debug!(round, "Negotiation round");

        let seller_prompt = format!(
            "Recent history:\n{}\n\nRespond as the Seller's Assistant.",
            history.join("\n")
        );
        let seller_response: String = seller_agent.prompt(seller_prompt).await?;

        tracing::info!(sender = "Seller-Assistant", message = %seller_response, "Seller response");
        history.push(format!("Seller-Assistant: {}", seller_response));

        let _ = event_tx
            .send(BusinessEvent::ChatMessage {
                conversation_id: conversation_id.clone(),
                listing_id: listing_id.clone(),
                sender: "Seller-Assistant".to_string(),
                content: seller_response.clone(),
                image_data: None,
                audio_data: None,
            })
            .await;

        if let Some(price) = extract_deal_price(&seller_response) {
            tracing::info!(price, "Deal reached via seller");
            let _ = event_tx
                .send(BusinessEvent::DealReached {
                    listing_id: listing_id.clone(),
                    buyer_id: "ai-buyer-bot".to_string(),
                    seller_id: "human-owner".to_string(),
                    final_price: price,
                })
                .await;
            break;
        }

        if seller_response.contains("REJECT") {
            tracing::info!("Seller rejected");
            break;
        }

        let buyer_prompt = format!(
            "Recent history:\n{}\n\nRespond as the Buyer.",
            history.join("\n")
        );
        let buyer_response: String = buyer_agent.prompt(buyer_prompt).await?;

        tracing::info!(sender = "Buyer-Bot", message = %buyer_response, "Buyer response");
        history.push(format!("Buyer-Bot: {}", buyer_response));

        let _ = event_tx
            .send(BusinessEvent::ChatMessage {
                conversation_id: conversation_id.clone(),
                listing_id: listing_id.clone(),
                sender: "Buyer-Bot".to_string(),
                content: buyer_response.clone(),
                image_data: None,
                audio_data: None,
            })
            .await;

        if let Some(price) = extract_deal_price(&buyer_response) {
            tracing::info!(price, "Deal reached via buyer");
            let _ = event_tx
                .send(BusinessEvent::DealReached {
                    listing_id: listing_id.clone(),
                    buyer_id: "ai-buyer-bot".to_string(),
                    seller_id: "human-owner".to_string(),
                    final_price: price,
                })
                .await;
            break;
        }

        if buyer_response.contains("REJECT") {
            tracing::info!("Buyer rejected");
            break;
        }
    }

    tracing::info!("Negotiation session finished");
    Ok(())
}

/// Improved price extraction - uses regex for robustness
fn extract_deal_price(text: &str) -> Option<i64> {
    // Look for DEAL: followed by a price pattern
    // Handles: "DEAL: 2000", "DEAL:2000", "DEAL: 2000 CNY", "DEAL: 2000.50"
    use regex::Regex;
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"(?i)DEAL:\s*(\d+(?:\.\d{1,2})?)").unwrap());

    if let Some(caps) = RE.captures(text) {
        if let Some(price_str) = caps.get(1) {
            let price_f: f64 = price_str.as_str().parse().ok()?;
            return Some(yuan_to_cents(price_f));
        }
    }
    None
}
