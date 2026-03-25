pub mod gemini;
pub mod minimax;

use crate::services::BusinessEvent;
use async_trait::async_trait;
use futures::Stream;
use rig::completion::Message;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Unified LLM provider interface.
///
/// Each concrete provider (Gemini, MiniMax) implements this trait,
/// providing agent creation with provider-specific types kept internal.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Create a RAG-enabled marketplace agent.
    ///
    /// Each provider owns its concrete vector store type and embedding model,
    /// hiding these implementation details from callers.
    async fn create_marketplace_agent(
        self: Arc<Self>,
        db_pool: &sqlx::PgPool,
        event_tx: mpsc::Sender<BusinessEvent>,
        current_user_id: Option<String>,
    ) -> anyhow::Result<Box<dyn MarketplaceAgent>>;

    /// Create a negotiation agent.
    async fn create_negotiate_agent(self: Arc<Self>) -> anyhow::Result<Box<dyn NegotiateAgent>>;
}

/// Marker trait for marketplace agents — erased via `Box<dyn MarketplaceAgent>`.
#[async_trait]
#[allow(dead_code)]
pub trait MarketplaceAgent: Send + Sync {
    async fn prompt(&self, msg: String) -> anyhow::Result<String>;
    async fn prompt_with_history(
        &self,
        msg: String,
        history: Vec<Message>,
    ) -> anyhow::Result<String>;

    /// Stream chat response tokens as they arrive.
    /// Returns a stream of text chunks and a final conversation_id.
    fn stream_chat(
        &self,
        msg: String,
        history: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, anyhow::Error>> + Send>>;
}

/// Marker trait for negotiation agents.
#[async_trait]
pub trait NegotiateAgent: Send + Sync {
    async fn prompt(&self, msg: String) -> anyhow::Result<String>;
}

/// Chinese preamble injected into all marketplace agents.
pub const PREAMBLE: &str = "\
你是一个校园二手交易平台的智能助手。

### 核心行为准则：
1. **区分信息来源**：
   - **用户输入**：用户通过对话直接告诉你的信息。
   - **库存上下文 (Store Context)**：通过 dynamic_context 提供的信息，它们来自平台数据库。
   - **禁止混淆**：绝对不要对用户说你刚才提供了XX项目的信息。如果信息来自上下文，请说根据平台目前的库存显示或我发现有一件...
2. **按需提供信息**：
   - 如果用户只是在聊天，不要罗列随机搜到的库存商品细节。只需介绍你的功能。
   - 只有当用户表现出购买意向、搜索意向或询问特定商品时，才引用库存上下文。
3. **功能边界**：
   - **卖东西**：调用 create_listing。
   - **买/搜东西**：优先使用 search_inventory 进行精准带条件的搜索；对于模糊浏览，使用动态上下文。
   - **管理**：通过 get_my_listings, update_listing, delete_listing 维护卖家的商品。
   - **交易**：用户确认要买时，调用 purchase_item 发起意向。

始终保持专业、友好、简洁，并明确区分你的知识库内容和用户实时输入。";

/// Negotiation agent preamble.
pub const NEGOTIATION_PREAMBLE: &str = "\
你是一个专业的AI谈判助手，擅长在二手交易中帮助用户优化交易价格。

你的职责是：
1. 分析卖家和买家的出价，找出共同点
2. 提出合理的中间价建议
3. 解释你的谈判逻辑
4. 逐步引导双方达成共识

记住：始终以友好的方式沟通，帮助双方达成公平交易。";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preamble_is_not_empty() {
        assert!(!PREAMBLE.is_empty());
        assert!(PREAMBLE.contains("校园二手交易平台"));
    }

    #[test]
    fn test_negotiation_preamble_is_not_empty() {
        assert!(!NEGOTIATION_PREAMBLE.is_empty());
        assert!(NEGOTIATION_PREAMBLE.contains("AI谈判助手"));
    }

    #[test]
    fn test_preamble_contains_core_behavior_guidelines() {
        // Verify preamble contains key behavior instructions
        assert!(PREAMBLE.contains("create_listing"));
        assert!(PREAMBLE.contains("search_inventory"));
        assert!(PREAMBLE.contains("purchase_item"));
    }

    #[test]
    fn test_negotiation_preamble_contains_pricing_guidance() {
        assert!(NEGOTIATION_PREAMBLE.contains("优化交易价格"));
        assert!(NEGOTIATION_PREAMBLE.contains("中间价建议"));
    }

    #[test]
    fn test_llm_provider_trait_objects_compile() {
        // Verify trait bounds are satisfied (this is a compile-time check)
        fn assert_send_sync<T: Send + Sync>() {}
        // These are marker traits but we verify the bounds compile
        assert_send_sync::<Box<dyn MarketplaceAgent>>();
        assert_send_sync::<Box<dyn NegotiateAgent>>();
    }
}
