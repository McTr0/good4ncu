use super::{MarketplaceAgent, NegotiateAgent, NEGOTIATION_PREAMBLE, PREAMBLE};
use crate::agents::models::Document;
use crate::agents::tools::{EmbedFn, ToolContext, ToolError};
use crate::services::BusinessEvent;
use async_trait::async_trait;
use rig::agent::Agent;
use rig::client::CompletionClient;
use rig::completion::{Message, Prompt};
use rig::embeddings::EmbeddingsBuilder;
use rig::providers::gemini;
use rig::providers::openai;
use rig::vector_store::InsertDocuments;
use rig_postgres::PostgresVectorStore;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct MiniMaxProvider {
    /// MiniMax OpenAI-compatible client for chat completions.
    chat_client: openai::Client<reqwest::Client>,
    /// Gemini client used only for embeddings (vector search + tool insertion).
    embedding_client: gemini::Client,
    model: String,
    embedding_dim: usize,
}

impl MiniMaxProvider {
    pub fn new(
        api_key: &str,
        base_url: Option<&str>,
        gemini_api_key: &str,
        embedding_dim: usize,
    ) -> anyhow::Result<Self> {
        let base_url = base_url.unwrap_or("https://api.minimaxi.com/v1");
        let chat_client = openai::Client::builder()
            .api_key(api_key)
            .base_url(base_url)
            .build()?;

        let reqwest_client = reqwest::Client::builder()
            .build()
            .expect("Failed to build reqwest client");
        let embedding_client = gemini::Client::builder()
            .api_key(gemini_api_key)
            .http_client(reqwest_client)
            .build()?;

        Ok(Self {
            chat_client,
            embedding_client,
            model: "MiniMax-M2.7".to_string(),
            embedding_dim,
        })
    }

    /// Build the RAG index for dynamic_context and the embed function for tools.
    ///
    /// Uses Gemini's embedding model for both RAG retrieval and tool insertion.
    /// Two separate PostgresVectorStore instances hit the same DB:
    /// - `rag_store`: passed to dynamic_context for RAG context retrieval
    /// - `embed_fn`: creates a FRESH store instance per call for insertion
    pub fn build_vector_store(
        &self,
        db_pool: &PgPool,
    ) -> (
        PostgresVectorStore<gemini::embedding::EmbeddingModel>,
        EmbedFn,
    ) {
        let embedding_model = gemini::embedding::EmbeddingModel::new(
            self.embedding_client.clone(),
            gemini::EMBEDDING_001,
            self.embedding_dim,
        );
        // RAG store: owned by dynamic_context (consumed at agent build time).
        let rag_store =
            PostgresVectorStore::with_defaults(embedding_model.clone(), db_pool.clone());

        // embed_fn creates a FRESH store instance per call for embedding + insertion.
        // Uses the same DB pool, so inserts are visible to rag_store queries.
        let db_pool_clone = db_pool.clone();
        let embedding_client_clone = self.embedding_client.clone();
        let dim = self.embedding_dim;

        let embed_fn: EmbedFn = Arc::new(move |content: String, listing_id: String| {
            let db_pool = db_pool_clone.clone();
            let embedding_client = embedding_client_clone.clone();
            let client_for_insert = embedding_client.clone();
            Box::pin(async move {
                let embedding_model = gemini::embedding::EmbeddingModel::new(
                    embedding_client,
                    gemini::EMBEDDING_001,
                    dim,
                );
                let document = Document {
                    id: listing_id,
                    content,
                };
                let embeddings = EmbeddingsBuilder::new(embedding_model)
                    .document(document)
                    .map_err(|e| ToolError(format!("Embedding builder error: {}", e)))?
                    .build()
                    .await
                    .map_err(|e| ToolError(format!("Embeddings API error: {}", e)))?;

                // Create a fresh store instance per call for insertion.
                // Both stores share the same DB, so inserts are immediately visible.
                let insert_store = PostgresVectorStore::with_defaults(
                    gemini::embedding::EmbeddingModel::new(
                        client_for_insert,
                        gemini::EMBEDDING_001,
                        dim,
                    ),
                    db_pool,
                );
                insert_store
                    .insert_documents(embeddings)
                    .await
                    .map_err(|e| ToolError(format!("Vector DB error: {:?}", e)))?;
                Ok(())
            })
        });

        (rag_store, embed_fn)
    }
}

#[async_trait]
impl super::LlmProvider for MiniMaxProvider {
    fn name(&self) -> &str {
        "minimax"
    }

    async fn create_marketplace_agent(
        self: Arc<Self>,
        db_pool: &sqlx::PgPool,
        event_tx: mpsc::Sender<BusinessEvent>,
        current_user_id: Option<String>,
    ) -> anyhow::Result<Box<dyn MarketplaceAgent>> {
        let (rag_store, embed_fn) = self.build_vector_store(db_pool);

        let ctx = ToolContext {
            db_pool: db_pool.clone(),
            embed_and_insert: embed_fn,
            event_tx,
            current_user_id,
            notification: crate::services::notification::NotificationService::new(db_pool.clone()),
        };

        let agent = self
            .chat_client
            .agent(&self.model)
            .preamble(PREAMBLE)
            .dynamic_context(3, rag_store)
            .tool(crate::agents::tools::CreateListingTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::SearchInventoryTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::GetListingDetailsTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::UpdateListingTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::DeleteListingTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::PurchaseItemIntentTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::NegotiateItemTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::GetMyListingsTool { ctx: ctx.clone() })
            .build();

        Ok(Box::new(MiniMaxMarketplaceAgent(agent)))
    }

    async fn create_negotiate_agent(self: Arc<Self>) -> anyhow::Result<Box<dyn NegotiateAgent>> {
        let agent = self
            .chat_client
            .agent(&self.model)
            .preamble(NEGOTIATION_PREAMBLE)
            .build();

        Ok(Box::new(MiniMaxNegotiateAgent(agent)))
    }
}

pub struct MiniMaxMarketplaceAgent(
    Agent<openai::responses_api::ResponsesCompletionModel<reqwest::Client>>,
);

#[async_trait]
impl MarketplaceAgent for MiniMaxMarketplaceAgent {
    async fn prompt(&self, msg: String) -> anyhow::Result<String> {
        Ok(self.0.prompt(msg).await?)
    }

    async fn prompt_with_history(
        &self,
        msg: String,
        history: Vec<Message>,
    ) -> anyhow::Result<String> {
        let mut h = history;
        let reply = self
            .0
            .prompt(rig::completion::Message::user(msg))
            .with_history(&mut h)
            .await?;
        Ok(reply)
    }
}

pub struct MiniMaxNegotiateAgent(
    Agent<openai::responses_api::ResponsesCompletionModel<reqwest::Client>>,
);

#[async_trait]
impl NegotiateAgent for MiniMaxNegotiateAgent {
    async fn prompt(&self, msg: String) -> anyhow::Result<String> {
        Ok(self.0.prompt(msg).await?)
    }
}
