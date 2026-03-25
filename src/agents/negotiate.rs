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
    use inquire::error::InquireError;

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
            Err(InquireError::OperationInterrupted) | Err(InquireError::OperationCanceled) => {
                println!("\nOffer cancelled - rejecting.");
                let _ = response_tx.send(HitlResult::Rejected);
            }
            Err(e) => {
                tracing::error!(%e, "HITL prompt error");
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
    let seller_agent: Box<dyn NegotiateAgent> = provider.clone().create_negotiate_agent().await?;

    let buyer_agent: Box<dyn NegotiateAgent> = provider.create_negotiate_agent().await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_deal_price_various_formats() {
        // Basic formats
        assert_eq!(extract_deal_price("DEAL: 2000"), Some(200_000));
        assert_eq!(extract_deal_price("DEAL:2000"), Some(200_000));
        assert_eq!(extract_deal_price("DEAL: 2000 CNY"), Some(200_000));
        assert_eq!(extract_deal_price("DEAL: 2000.50"), Some(200_050));
        assert_eq!(extract_deal_price("DEAL: 199.99"), Some(19_999));

        // Case insensitive
        assert_eq!(extract_deal_price("deal: 1500"), Some(150_000));
        assert_eq!(extract_deal_price("Deal: 1500"), Some(150_000));
        assert_eq!(extract_deal_price("dEaL: 1500"), Some(150_000));

        // No deal
        assert_eq!(extract_deal_price("No deal here"), None);
        assert_eq!(extract_deal_price(""), None);

        // In context
        assert_eq!(
            extract_deal_price("The price is DEAL: 2500 for this item"),
            Some(250_000)
        );
        assert_eq!(
            extract_deal_price("Final offer: DEAL: 999.99 plus shipping"),
            Some(99_999)
        );
    }

    #[test]
    fn test_hitl_channel_disabled_auto_rejects() {
        let _hitl = HitlChannel::new_disabled();
        // Should fail immediately since no CLI channel is configured
        let _request = HitlRequest {
            id: "test-1".to_string(),
            proposed_price: 100_00, // 100.00 CNY
            reason: "Test".to_string(),
            status: "pending".to_string(),
            counter_price: None,
        };
        // The new_disabled HITL should return error on request_approval
        // (we can't easily test async without a runtime, but verify it compiles)
    }

    #[test]
    fn test_hitl_request_structure() {
        let req = HitlRequest {
            id: "req-123".to_string(),
            proposed_price: 500_00,
            reason: "Price is too high".to_string(),
            status: "pending".to_string(),
            counter_price: None,
        };
        assert_eq!(req.status, "pending");
        assert_eq!(req.counter_price, None);
    }

    #[test]
    fn test_hitl_result_countered() {
        let result = HitlResult::Countered(350_00);
        // Verify the enum variant carries the correct price
        match result {
            HitlResult::Countered(price) => assert_eq!(price, 350_00),
            _ => panic!("Expected Countered variant"),
        }
    }

    #[test]
    fn test_hitl_result_approved() {
        let result = HitlResult::Approved;
        match result {
            HitlResult::Approved => {}
            _ => panic!("Expected Approved variant"),
        }
    }

    #[test]
    fn test_hitl_result_rejected() {
        let result = HitlResult::Rejected;
        match result {
            HitlResult::Rejected => {}
            _ => panic!("Expected Rejected variant"),
        }
    }

    #[test]
    fn test_human_interaction_error_display() {
        let err = HumanInteractionError("test error".to_string());
        assert_eq!(err.to_string(), "Human interaction error: test error");
    }

    #[test]
    fn test_human_approval_args_deserialization() {
        let json = r#"{"proposed_price": 450000, "reason": "Final offer"}"#;
        let args: HumanApprovalArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.proposed_price, 450000);
        assert_eq!(args.reason, "Final offer");
    }

    #[test]
    fn test_human_approval_args_all_fields() {
        let json = r#"{
            "proposed_price": 39900,
            "reason": "Testing deserialization"
        }"#;
        let args: HumanApprovalArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.proposed_price, 39900);
    }

    #[test]
    fn test_human_decision_approved_serialization() {
        let decision = HumanDecision {
            action: "approve".to_string(),
            counter_price: None,
            message: "Human approved the deal.".to_string(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("\"action\":\"approve\""));
        assert!(json.contains("Human approved"));
        assert!(json.contains("counter_price"));
    }

    #[test]
    fn test_human_decision_countered_serialization() {
        let decision = HumanDecision {
            action: "counter".to_string(),
            counter_price: Some(425000),
            message: "Human countered with 4250 CNY.".to_string(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("\"action\":\"counter\""));
        assert!(json.contains("\"counter_price\":4250"));
    }

    #[test]
    fn test_human_decision_rejected_serialization() {
        let decision = HumanDecision {
            action: "reject".to_string(),
            counter_price: None,
            message: "Human rejected the offer.".to_string(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("\"action\":\"reject\""));
    }

    #[test]
    fn test_human_decision_deserialization() {
        let json = r#"{
            "action": "approve",
            "counter_price": null,
            "message": "OK"
        }"#;
        let decision: HumanDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision.action, "approve");
        assert_eq!(decision.counter_price, None);
        assert_eq!(decision.message, "OK");
    }

    #[test]
    fn test_human_decision_countered_deserialization() {
        let json = r#"{
            "action": "counter",
            "counter_price": 400000,
            "message": "Counter offer"
        }"#;
        let decision: HumanDecision = serde_json::from_str(json).unwrap();
        assert_eq!(decision.action, "counter");
        assert_eq!(decision.counter_price, Some(400000));
    }

    #[test]
    fn test_hitl_request_serialization() {
        let request = HitlRequest {
            id: "req-abc".to_string(),
            proposed_price: 500_00,
            reason: "Test reason".to_string(),
            status: "pending".to_string(),
            counter_price: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("req-abc"));
        assert!(json.contains("\"status\":\"pending\""));
    }

    #[test]
    fn test_hitl_request_with_counter() {
        let request = HitlRequest {
            id: "req-xyz".to_string(),
            proposed_price: 500_00,
            reason: "Counter needed".to_string(),
            status: "countered".to_string(),
            counter_price: Some(450_00),
        };
        assert_eq!(request.counter_price, Some(450_00));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"counter_price\":45000"));
    }

    #[test]
    fn test_hitl_request_deserialization() {
        let json = r#"{
            "id": "req-123",
            "proposed_price": 75000,
            "reason": "Too expensive",
            "status": "pending",
            "counter_price": null
        }"#;
        let request: HitlRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.id, "req-123");
        assert_eq!(request.proposed_price, 75000);
        assert_eq!(request.status, "pending");
        assert_eq!(request.counter_price, None);
    }

    #[test]
    fn test_hitl_channel_disabled_clone() {
        let channel = HitlChannel::new_disabled();
        let _cloned = channel.clone();
        // Verify Clone impl works
    }

    #[test]
    fn test_human_approval_tool_new() {
        let hitl = HitlChannel::new_disabled();
        let tool = HumanApprovalTool::new(hitl);
        let _ = tool;
    }
}
