use crate::services::BusinessEvent;
use crate::utils::cents_to_yuan;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Shared context for all tools
// ---------------------------------------------------------------------------

/// Type alias for the embedding function passed to tools.
pub type EmbedFn = Arc<
    dyn Fn(String, String) -> Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>
        + Send
        + Sync,
>;

/// Shared dependencies injected into every marketplace tool.
#[derive(Clone)]
pub struct ToolContext {
    /// PostgreSQL pool — serves both relational data and vector data (pgvector).
    pub db_pool: PgPool,
    /// Callback to embed and insert a listing into the vector store.
    /// This encapsulates the provider-specific embedding model type.
    pub embed_and_insert: EmbedFn,
    pub event_tx: mpsc::Sender<BusinessEvent>,
    pub current_user_id: Option<String>,
}

/// Unified error type for all marketplace tools.
#[derive(Debug, thiserror::Error)]
#[error("Tool error: {0}")]
pub struct ToolError(pub String);

// ---------------------------------------------------------------------------
// 1. CreateListingTool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateListingArgs {
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: u8,
    pub suggested_price_cny: i64,
    pub defects: Vec<String>,
    pub original_description: String,
}

#[derive(Clone)]
pub struct CreateListingTool {
    pub ctx: ToolContext,
}

impl Tool for CreateListingTool {
    const NAME: &'static str = "create_listing";
    type Error = ToolError;
    type Args = CreateListingArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "create_listing".to_string(),
            description:
                "Creates a new secondhand item listing. Use when a user wants to sell something."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "A short title for the item" },
                    "category": { "type": "string", "description": "Item category" },
                    "brand": { "type": "string", "description": "Item brand" },
                    "condition_score": { "type": "integer", "description": "Condition from 1 to 10" },
                    "suggested_price_cny": { "type": "number", "description": "Price in CNY" },
                    "defects": { "type": "array", "items": { "type": "string" }, "description": "Any defects" },
                    "original_description": { "type": "string", "description": "User's original description" }
                },
                "required": ["title", "category", "brand", "condition_score", "suggested_price_cny", "defects", "original_description"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let owner =
            self.ctx.current_user_id.clone().ok_or_else(|| {
                ToolError("Authentication required. Please login first.".to_string())
            })?;
        let listing_id = uuid::Uuid::new_v4().to_string();
        let defects_json =
            serde_json::to_string(&args.defects).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "INSERT INTO inventory (id, title, category, brand, condition_score, suggested_price_cny, defects, description, owner_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&listing_id)
        .bind(&args.title)
        .bind(&args.category)
        .bind(&args.brand)
        .bind(args.condition_score as i64)
        .bind(args.suggested_price_cny)
        .bind(&defects_json)
        .bind(&args.original_description)
        .bind(&owner)
        .execute(&self.ctx.db_pool)
        .await
        .map_err(|e| ToolError(format!("DB insert error: {}", e)))?;

        let content_to_embed = format!(
            "Title: {}\nCategory: {}\nBrand: {}\nCondition: {}/10\nDescription: {}",
            args.title, args.category, args.brand, args.condition_score, args.original_description
        );
        let embed_fn = self.ctx.embed_and_insert.clone();
        embed_fn(content_to_embed, listing_id.clone())
            .await
            .map_err(|e| ToolError(format!("Embedding error: {}", e)))?;

        Ok(format!(
            "Successfully created listing '{}' (ID: {}, Price: {} CNY, Owner: {})",
            args.title,
            listing_id,
            cents_to_yuan(args.suggested_price_cny),
            owner
        ))
    }
}

// ---------------------------------------------------------------------------
// 2. SearchInventoryTool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SearchInventoryArgs {
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub max_price: Option<i64>,
    pub min_condition: Option<u8>,
}

#[derive(Clone)]
pub struct SearchInventoryTool {
    pub ctx: ToolContext,
}

impl Tool for SearchInventoryTool {
    const NAME: &'static str = "search_inventory";
    type Error = ToolError;
    type Args = SearchInventoryArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_inventory".to_string(),
            description: "Searches the marketplace inventory with optional filters. Use when a user wants to find items with specific criteria like price range, category, or condition.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "Search keyword to match against title or description" },
                    "category": { "type": "string", "description": "Filter by category" },
                    "max_price": { "type": "number", "description": "Maximum price in CNY" },
                    "min_condition": { "type": "integer", "description": "Minimum condition score (1-10)" }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut sql = String::from("SELECT id, title, brand, category, condition_score, suggested_price_cny FROM inventory WHERE status = 'active'");
        let mut param_idx: usize = 1;

        if args.keyword.is_some() {
            sql.push_str(&format!(
                " AND (title LIKE ${} OR description LIKE ${})",
                param_idx,
                param_idx + 1
            ));
            param_idx += 2;
        }
        if args.category.is_some() {
            sql.push_str(&format!(" AND category LIKE ${}", param_idx));
            param_idx += 1;
        }
        if args.max_price.is_some() {
            sql.push_str(&format!(" AND suggested_price_cny <= ${}", param_idx));
        }
        if let Some(min_c) = args.min_condition {
            sql.push_str(&format!(" AND condition_score >= {}", min_c));
        }
        sql.push_str(" LIMIT 10");

        let mut query = sqlx::query_as::<_, InventoryRow>(&sql);

        if let Some(ref kw) = args.keyword {
            query = query.bind(format!("%{}%", kw)).bind(format!("%{}%", kw));
        }
        if let Some(ref cat) = args.category {
            query = query.bind(format!("%{}%", cat));
        }
        if let Some(max_p) = args.max_price {
            query = query.bind(max_p);
        }

        let rows = query
            .fetch_all(&self.ctx.db_pool)
            .await
            .map_err(|e| ToolError(format!("Search query error: {}", e)))?;

        if rows.is_empty() {
            return Ok("No items found matching your criteria.".to_string());
        }

        let mut result = format!("Found {} item(s):\n", rows.len());
        for r in &rows {
            result.push_str(&format!(
                "- [{}] {} (Brand: {}, Category: {}, Condition: {}/10, Price: {} CNY)\n",
                r.id,
                r.title,
                r.brand,
                r.category,
                r.condition_score,
                cents_to_yuan(r.suggested_price_cny)
            ));
        }
        Ok(result)
    }
}

#[derive(sqlx::FromRow)]
struct InventoryRow {
    id: String,
    title: String,
    brand: String,
    category: String,
    condition_score: i64,
    suggested_price_cny: i64,
}

// ---------------------------------------------------------------------------
// 3. GetListingDetailsTool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct GetListingDetailsArgs {
    pub listing_id: String,
}

#[derive(Clone)]
pub struct GetListingDetailsTool {
    pub ctx: ToolContext,
}

impl Tool for GetListingDetailsTool {
    const NAME: &'static str = "get_listing_details";
    type Error = ToolError;
    type Args = GetListingDetailsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_listing_details".to_string(),
            description: "Gets the full details of a specific listing by its ID.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "listing_id": { "type": "string", "description": "The ID of the listing" }
                },
                "required": ["listing_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let row = sqlx::query_as::<_, FullListingRow>(
            "SELECT id, title, category, brand, condition_score, suggested_price_cny, defects, description, owner_id, status FROM inventory WHERE id = $1",
        )
        .bind(&args.listing_id)
        .fetch_optional(&self.ctx.db_pool)
        .await
        .map_err(|e| ToolError(format!("Query error: {}", e)))?;

        match row {
            Some(r) => {
                // Only show owner_id if the current user is the owner
                let owner_display = if Some(&r.owner_id) == self.ctx.current_user_id.as_ref() {
                    r.owner_id.clone()
                } else {
                    "[hidden]".to_string()
                };
                Ok(format!(
                    "Listing Details:\n\
                     ID: {}\nTitle: {}\nCategory: {}\nBrand: {}\n\
                     Condition: {}/10\nPrice: {} CNY\nDefects: {}\n\
                     Description: {}\nOwner: {}\nStatus: {}",
                    r.id,
                    r.title,
                    r.category,
                    r.brand,
                    r.condition_score,
                    cents_to_yuan(r.suggested_price_cny),
                    r.defects,
                    r.description.unwrap_or_default(),
                    owner_display,
                    r.status
                ))
            }
            None => Ok(format!("No listing found with ID: {}", args.listing_id)),
        }
    }
}

#[derive(sqlx::FromRow)]
struct FullListingRow {
    id: String,
    title: String,
    category: String,
    brand: String,
    condition_score: i64,
    suggested_price_cny: i64,
    defects: String,
    description: Option<String>,
    owner_id: String,
    status: String,
}

// ---------------------------------------------------------------------------
// 4. UpdateListingTool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct UpdateListingArgs {
    pub listing_id: String,
    pub new_price: Option<i64>,
    pub new_title: Option<String>,
    pub new_description: Option<String>,
}

#[derive(Clone)]
pub struct UpdateListingTool {
    pub ctx: ToolContext,
}

impl Tool for UpdateListingTool {
    const NAME: &'static str = "update_listing";
    type Error = ToolError;
    type Args = UpdateListingArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "update_listing".to_string(),
            description: "Updates a listing's price, title, or description. Use when a seller wants to modify their listing.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "listing_id": { "type": "string", "description": "The listing ID to update" },
                    "new_price": { "type": "number", "description": "New price in CNY" },
                    "new_title": { "type": "string", "description": "New title" },
                    "new_description": { "type": "string", "description": "New description" }
                },
                "required": ["listing_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let owner_id =
            self.ctx.current_user_id.clone().ok_or_else(|| {
                ToolError("Authentication required. Please login first.".to_string())
            })?;

        if args.new_price.is_none() && args.new_title.is_none() && args.new_description.is_none() {
            return Ok("No fields to update were provided.".to_string());
        }

        let mut qb = sqlx::QueryBuilder::new("UPDATE inventory SET ");
        let mut sep = qb.separated(", ");

        if let Some(price) = args.new_price {
            sep.push("suggested_price_cny = ");
            sep.push_bind_unseparated(price);
        }
        if let Some(ref title) = args.new_title {
            sep.push("title = ");
            sep.push_bind_unseparated(title);
        }
        if let Some(ref desc) = args.new_description {
            sep.push("description = ");
            sep.push_bind_unseparated(desc);
        }

        qb.push(" WHERE id = ");
        qb.push_bind(&args.listing_id);
        qb.push(" AND owner_id = ");
        qb.push_bind(&owner_id);
        qb.push(" AND status = 'active'");

        let result = qb
            .build()
            .execute(&self.ctx.db_pool)
            .await
            .map_err(|e| ToolError(format!("Update error: {}", e)))?;

        if result.rows_affected() == 0 {
            Ok(format!(
                "No active listing found with ID: {} (or you don't own it)",
                args.listing_id
            ))
        } else {
            Ok(format!("Successfully updated listing {}", args.listing_id))
        }
    }
}

// ---------------------------------------------------------------------------
// 5. DeleteListingTool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct DeleteListingArgs {
    pub listing_id: String,
}

#[derive(Clone)]
pub struct DeleteListingTool {
    pub ctx: ToolContext,
}

impl Tool for DeleteListingTool {
    const NAME: &'static str = "delete_listing";
    type Error = ToolError;
    type Args = DeleteListingArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "delete_listing".to_string(),
            description: "Removes (soft-deletes) a listing from the marketplace. Use when a seller wants to take down their item.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "listing_id": { "type": "string", "description": "The listing ID to remove" }
                },
                "required": ["listing_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let owner_id =
            self.ctx.current_user_id.clone().ok_or_else(|| {
                ToolError("Authentication required. Please login first.".to_string())
            })?;

        let result = sqlx::query(
            "UPDATE inventory SET status = 'deleted' WHERE id = $1 AND owner_id = $2 AND status = 'active'",
        )
        .bind(&args.listing_id)
        .bind(&owner_id)
        .execute(&self.ctx.db_pool)
        .await
        .map_err(|e| ToolError(format!("Delete error: {}", e)))?;

        if result.rows_affected() == 0 {
            return Ok(format!(
                "No active listing found with ID: {} (or you don't own it)",
                args.listing_id
            ));
        }

        // Sync vector store: remove stale embedding so RAG won't surface deleted listings.
        // pgvector stores documents in the same 'documents' table, so we use SQL DELETE.
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(&args.listing_id)
            .execute(&self.ctx.db_pool)
            .await
            .ok(); // Fire-and-forget: vector cleanup failure is non-fatal

        Ok(format!("Successfully removed listing {}", args.listing_id))
    }
}

// ---------------------------------------------------------------------------
// 6. PurchaseItemIntentTool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PurchaseItemIntentArgs {
    pub listing_id: String,
    pub offered_price: i64,
}

#[derive(Clone)]
pub struct PurchaseItemIntentTool {
    pub ctx: ToolContext,
}

impl Tool for PurchaseItemIntentTool {
    const NAME: &'static str = "purchase_item";
    type Error = ToolError;
    type Args = PurchaseItemIntentArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "purchase_item".to_string(),
            description: "Initiates a purchase intent for an item. This triggers the order creation process. Use when a user confirms they want to buy a specific item.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "listing_id": { "type": "string", "description": "The listing ID to purchase" },
                    "offered_price": { "type": "number", "description": "The offered purchase price in CNY" }
                },
                "required": ["listing_id", "offered_price"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Verify the listing exists and is active
        let listing = sqlx::query_as::<_, ListingCheckRow>(
            "SELECT id, owner_id, suggested_price_cny, status FROM inventory WHERE id = $1",
        )
        .bind(&args.listing_id)
        .fetch_optional(&self.ctx.db_pool)
        .await
        .map_err(|e| ToolError(format!("Query error: {}", e)))?;

        let listing = match listing {
            Some(l) => l,
            None => return Ok(format!("No listing found with ID: {}", args.listing_id)),
        };

        if listing.status != "active" {
            return Ok(format!(
                "Listing {} is no longer available (status: {})",
                args.listing_id, listing.status
            ));
        }

        // Require authentication
        let buyer_id =
            self.ctx.current_user_id.clone().ok_or_else(|| {
                ToolError("Authentication required. Please login first.".to_string())
            })?;

        // Cannot buy your own listing
        if buyer_id == listing.owner_id {
            return Err(ToolError(
                "You cannot purchase your own listing.".to_string(),
            ));
        }

        // Emit DealReached event to trigger order creation
        self.ctx
            .event_tx
            .send(BusinessEvent::DealReached {
                listing_id: args.listing_id.clone(),
                buyer_id: buyer_id.clone(),
                seller_id: listing.owner_id.clone(),
                final_price: args.offered_price,
            })
            .await
            .map_err(|e| {
                tracing::error!(%e, "Failed to emit DealReached event");
                ToolError(format!("Event bus error: {}", e))
            })?;

        Ok(format!(
            "Purchase initiated! Order is being created for listing '{}'. Buyer: {}, Seller: {}, Price: {} CNY",
            args.listing_id, buyer_id, listing.owner_id, args.offered_price
        ))
    }
}

#[derive(sqlx::FromRow)]
struct ListingCheckRow {
    #[sqlx(rename = "id")]
    _id: String,
    owner_id: String,
    #[sqlx(rename = "suggested_price_cny")]
    _suggested_price_cny: i64,
    status: String,
}

// ---------------------------------------------------------------------------
// 7. GetMyListingsTool
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct GetMyListingsArgs {}

#[derive(Clone)]
pub struct GetMyListingsTool {
    pub ctx: ToolContext,
}

impl Tool for GetMyListingsTool {
    const NAME: &'static str = "get_my_listings";
    type Error = ToolError;
    type Args = GetMyListingsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_my_listings".to_string(),
            description: "Retrieves all listings owned by the currently authenticated user. Use when the user wants to see or manage their own items.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let owner_id =
            self.ctx.current_user_id.clone().ok_or_else(|| {
                ToolError("Authentication required. Please login first.".to_string())
            })?;

        let rows = sqlx::query_as::<_, MyListingRow>(
            "SELECT id, title, status, suggested_price_cny FROM inventory WHERE owner_id = $1 ORDER BY status",
        )
        .bind(&owner_id)
        .fetch_all(&self.ctx.db_pool)
        .await
        .map_err(|e| ToolError(format!("Query error: {}", e)))?;

        if rows.is_empty() {
            return Ok(format!("No listings found for user: {}", owner_id));
        }

        let mut result = format!("Your listings ({} total):\n", rows.len());
        for r in &rows {
            let status_emoji = match r.status.as_str() {
                "active" => "\u{1F7E2}",
                "sold" => "\u{1F534}",
                "deleted" => "\u{26AB}",
                _ => "\u{26AA}",
            };
            result.push_str(&format!(
                "{} [{}] {} - {} CNY ({})\n",
                status_emoji,
                r.id,
                r.title,
                cents_to_yuan(r.suggested_price_cny),
                r.status
            ));
        }
        Ok(result)
    }
}

#[derive(sqlx::FromRow)]
struct MyListingRow {
    id: String,
    title: String,
    status: String,
    suggested_price_cny: i64,
}
