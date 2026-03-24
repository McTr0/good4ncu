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
use rig::vector_store::InsertDocuments;
use rig_postgres::PostgresVectorStore;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct GeminiProvider {
    client: gemini::Client,
    embedding_dim: usize,
}

impl GeminiProvider {
    pub fn new(api_key: &str, embedding_dim: usize) -> anyhow::Result<Self> {
        let reqwest_client = reqwest::Client::builder()
            .build()
            .expect("Failed to build reqwest client");

        let client = gemini::Client::builder()
            .api_key(api_key)
            .http_client(reqwest_client)
            .build()?;

        Ok(Self {
            client,
            embedding_dim,
        })
    }

    /// Build the RAG index for dynamic_context and the embed function for tools.
    ///
    /// Two separate PostgresVectorStore instances are created:
    /// - `rag_store`: passed to dynamic_context for RAG context retrieval
    /// - `embed_fn`: captures db_pool and creates a FRESH store instance per call
    ///   to insert embeddings; since both stores use the same DB connection,
    ///   inserts ARE visible to the rag_store's queries.
    pub fn build_vector_store(
        &self,
        db_pool: &PgPool,
    ) -> (
        PostgresVectorStore<gemini::embedding::EmbeddingModel>,
        EmbedFn,
    ) {
        let embedding_model = gemini::embedding::EmbeddingModel::new(
            self.client.clone(),
            gemini::EMBEDDING_001,
            self.embedding_dim,
        );
        // RAG store: owned by dynamic_context (consumed at agent build time).
        // This store handles context retrieval (top_n queries).
        let rag_store =
            PostgresVectorStore::with_defaults(embedding_model.clone(), db_pool.clone());

        // embed_fn creates a FRESH store instance per call.
        // This is safe because both this store and rag_store share the same
        // underlying Postgres DB (same db_pool), so inserts are visible to both.
        let db_pool_clone = db_pool.clone();
        let client_clone = self.client.clone();
        let dim = self.embedding_dim;

        let embed_fn: EmbedFn = Arc::new(move |content: String, listing_id: String| {
            let db_pool = db_pool_clone.clone();
            let client = client_clone.clone();
            let client_for_insert = client.clone();
            Box::pin(async move {
                let embedding_model =
                    gemini::embedding::EmbeddingModel::new(client, gemini::EMBEDDING_001, dim);
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
                // This store hits the same DB as rag_store, so inserts are visible.
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
impl super::LlmProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn create_marketplace_agent(
        self: Arc<Self>,
        db_pool: &PgPool,
        event_tx: mpsc::Sender<BusinessEvent>,
        current_user_id: Option<String>,
    ) -> anyhow::Result<Box<dyn MarketplaceAgent>> {
        let (rag_store, embed_fn) = self.build_vector_store(db_pool);

        let ctx = ToolContext {
            db_pool: db_pool.clone(),
            embed_and_insert: embed_fn,
            event_tx,
            current_user_id,
        };

        let agent = self
            .client
            .agent("gemini-3-flash-preview")
            .preamble(PREAMBLE)
            .dynamic_context(3, rag_store)
            .tool(crate::agents::tools::CreateListingTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::SearchInventoryTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::GetListingDetailsTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::UpdateListingTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::DeleteListingTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::PurchaseItemIntentTool { ctx: ctx.clone() })
            .tool(crate::agents::tools::GetMyListingsTool { ctx: ctx.clone() })
            .build();

        Ok(Box::new(GeminiMarketplaceAgent(agent)))
    }

    async fn create_negotiate_agent(self: Arc<Self>) -> anyhow::Result<Box<dyn NegotiateAgent>> {
        let agent = self
            .client
            .agent("gemini-3-flash-preview")
            .preamble(NEGOTIATION_PREAMBLE)
            .build();

        Ok(Box::new(GeminiNegotiateAgent(agent)))
    }
}

pub struct GeminiMarketplaceAgent(Agent<gemini::completion::CompletionModel<reqwest::Client>>);

#[async_trait]
impl MarketplaceAgent for GeminiMarketplaceAgent {
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
            .prompt(Message::user(msg))
            .with_history(&mut h)
            .await?;
        Ok(reply)
    }
}

pub struct GeminiNegotiateAgent(Agent<gemini::completion::CompletionModel<reqwest::Client>>);

#[async_trait]
impl NegotiateAgent for GeminiNegotiateAgent {
    async fn prompt(&self, msg: String) -> anyhow::Result<String> {
        Ok(self.0.prompt(msg).await?)
    }
}
