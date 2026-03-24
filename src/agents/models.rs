use rig::Embed;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Structured extraction result for a secondhand item listing.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone)]
pub struct ListingDetails {
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: u8,
    pub suggested_price_cny: i64,
    pub defects: Vec<String>,
}

/// A document stored in the PostgreSQL vector database for semantic search.
/// Document must implement Embed so EmbeddingsBuilder can extract its content.
#[derive(Embed, Serialize, Deserialize, Clone, Debug)]
pub struct Document {
    pub id: String,
    #[embed]
    pub content: String,
}
