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
    dyn Fn(
            String,
            String,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<(), ToolError>> + Send>>
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
    /// Notification service for sending in-app alerts (e.g., negotiation requests).
    pub notification: crate::services::notification::NotificationService,
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
            description: "发布新的二手商品。当用户想要出售商品时使用。".to_string(),
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
        let owner = self
            .ctx
            .current_user_id
            .clone()
            .ok_or_else(|| ToolError("请先登录再进行操作".to_string()))?;
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
            description: "搜索商品列表，支持关键词、分类、价格区间筛选。当用户想找特定商品时使用。"
                .to_string(),
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
        // Reject oversized keywords before they can cause performance issues with LIKE on large tables.
        const MAX_KEYWORD_LEN: usize = 200;
        if let Some(ref kw) = args.keyword {
            if kw.len() > MAX_KEYWORD_LEN {
                return Err(ToolError(format!(
                    "搜索关键词不能超过{}个字符",
                    MAX_KEYWORD_LEN
                )));
            }
        }

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
            param_idx += 1;
        }
        if args.min_condition.is_some() {
            sql.push_str(&format!(" AND condition_score >= ${}", param_idx));
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
        if let Some(min_c) = args.min_condition {
            query = query.bind(min_c as i32);
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
            description: "获取指定商品的完整详情。".to_string(),
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
            description: "修改商品的价格、标题或描述。当卖家想更新自己的商品信息时使用。"
                .to_string(),
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
        let owner_id = self
            .ctx
            .current_user_id
            .clone()
            .ok_or_else(|| ToolError("请先登录再进行操作".to_string()))?;

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
            description: "删除（软删除）一个商品。当卖家想下架自己的商品时使用。".to_string(),
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
        let owner_id = self
            .ctx
            .current_user_id
            .clone()
            .ok_or_else(|| ToolError("请先登录再进行操作".to_string()))?;

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
            description: "发起购买意向，创建订单。当用户确认购买某个商品时使用。".to_string(),
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
        // Verify the listing exists and is active — use FOR UPDATE to lock this row
        // and prevent a TOCTOU race where two concurrent purchases both see
        // status='active' and both emit DealReached events.
        let listing = sqlx::query_as::<_, ListingCheckRow>(
            "SELECT id, owner_id, suggested_price_cny, status FROM inventory WHERE id = $1 FOR UPDATE",
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
        let buyer_id = self
            .ctx
            .current_user_id
            .clone()
            .ok_or_else(|| ToolError("请先登录再进行操作".to_string()))?;

        // Cannot buy your own listing
        if buyer_id == listing.owner_id {
            return Err(ToolError("不能购买自己发布的商品".to_string()));
        }

        // Validate offered price is within reasonable range of suggested price (±50%).
        // This prevents both unrealistic lowballs and accidentally overpaying.
        const PRICE_TOLERANCE: f64 = 0.50;
        let min_price = (listing.suggested_price_cny as f64 * (1.0 - PRICE_TOLERANCE)) as i64;
        let max_price = (listing.suggested_price_cny as f64 * (1.0 + PRICE_TOLERANCE)) as i64;
        if args.offered_price < min_price || args.offered_price > max_price {
            return Err(ToolError(format!(
                "出价 ¥{:.2} 不在合理范围内（¥{:.2} - ¥{:.2}）。商品标价 ¥{:.2}。",
                cents_to_yuan(args.offered_price),
                cents_to_yuan(min_price),
                cents_to_yuan(max_price),
                cents_to_yuan(listing.suggested_price_cny),
            )));
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

// ---------------------------------------------------------------------------
// 6b. NegotiateItemTool —发起还价请求，卖家 HITL 确认
// ---------------------------------------------------------------------------

/// Args for the negotiate_item tool.
/// The seller will receive a notification and must approve/reject/counter via
/// PATCH /api/negotiations/{id}/respond. The deal only proceeds if the seller approves.
#[derive(Deserialize)]
pub struct NegotiateItemArgs {
    /// The listing the buyer wants to negotiate on
    pub listing_id: String,
    /// The buyer's proposed price (in CNY cents)
    pub offered_price: i64,
    /// Short reason for the offer (e.g., "lightly used", "market price dropped")
    pub reason: String,
}

#[derive(Clone)]
pub struct NegotiateItemTool {
    pub ctx: ToolContext,
}

impl Tool for NegotiateItemTool {
    const NAME: &'static str = "negotiate_item";
    type Error = ToolError;
    type Args = NegotiateItemArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "发起还价请求。买家对某件商品提出还价时使用，系统会通知卖家审批。只有卖家批准后才会创建订单。注意：不要对已经active的同一商品重复发起还价。".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "listing_id": { "type": "string", "description": "The listing ID to negotiate on" },
                    "offered_price": { "type": "number", "description": "The offered price in CNY cents" },
                    "reason": { "type": "string", "description": "Short reason for the offer" }
                },
                "required": ["listing_id", "offered_price", "reason"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let buyer_id = self
            .ctx
            .current_user_id
            .clone()
            .ok_or_else(|| ToolError("请先登录再进行操作".to_string()))?;

        // Fetch the listing to get the seller and check it's active
        let listing_row = sqlx::query_as::<_, ListingCheckRow>(
            "SELECT id, owner_id, suggested_price_cny, status FROM inventory WHERE id = $1",
        )
        .bind(&args.listing_id)
        .fetch_optional(&self.ctx.db_pool)
        .await
        .map_err(|e| ToolError(format!("DB error: {}", e)))?;

        let listing = match listing_row {
            Some(l) => l,
            None => return Ok(format!("No listing found with ID: {}", args.listing_id)),
        };

        if listing.status != "active" {
            return Ok(format!("商品 {} 已下架或售出，无法还价", args.listing_id));
        }

        if buyer_id == listing.owner_id {
            return Err(ToolError("不能对自己的商品发起还价".to_string()));
        }

        // Validate offered price is within a reasonable range (±50% of asking price)
        const PRICE_TOLERANCE: f64 = 0.50;
        let min_price = (listing.suggested_price_cny as f64 * (1.0 - PRICE_TOLERANCE)) as i64;
        let max_price = (listing.suggested_price_cny as f64 * (1.0 + PRICE_TOLERANCE)) as i64;
        if args.offered_price < min_price || args.offered_price > max_price {
            return Err(ToolError(format!(
                "还价 ¥{:.2} 不在合理范围 ¥{:.2} - ¥{:.2} 内",
                cents_to_yuan(args.offered_price),
                cents_to_yuan(min_price),
                cents_to_yuan(max_price),
            )));
        }

        // Check if there's already a pending negotiation for this buyer+listing
        let existing = sqlx::query(
            "SELECT id FROM hitl_requests WHERE listing_id = $1 AND buyer_id = $2 AND status = 'pending'",
        )
        .bind(&args.listing_id)
        .bind(&buyer_id)
        .fetch_optional(&self.ctx.db_pool)
        .await
        .map_err(|e| ToolError(format!("DB error: {}", e)))?;
        if existing.is_some() {
            return Ok("您已对该商品发起过还价，请等待卖家响应后再发起新还价".to_string());
        }

        // Create the HITL request in the database
        let hitl_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO hitl_requests
               (id, listing_id, buyer_id, seller_id, proposed_price, reason, status, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, 'pending', CURRENT_TIMESTAMP + INTERVAL '48 hours')"#,
        )
        .bind(&hitl_id)
        .bind(&args.listing_id)
        .bind(&buyer_id)
        .bind(&listing.owner_id)
        .bind(args.offered_price)
        .bind(&args.reason)
        .execute(&self.ctx.db_pool)
        .await
        .map_err(|e| ToolError(format!("DB error: {}", e)))?;

        // Notify the seller immediately
        let _ = self
            .ctx
            .notification
            .create(
                &listing.owner_id,
                "negotiation_request",
                "有新的还价请求",
                &format!(
                    "买家出价 ¥{:.2}，理由：{}",
                    cents_to_yuan(args.offered_price),
                    args.reason
                ),
                Some(&hitl_id),
                Some(&args.listing_id),
            )
            .await;

        Ok(format!(
            "您的还价 ¥{:.2} 已发送给卖家，等待确认中。\
             卖家同意后订单将自动创建。\
             请留意通知。",
            cents_to_yuan(args.offered_price)
        ))
    }
}

#[derive(sqlx::FromRow)]
struct ListingCheckRow {
    #[sqlx(rename = "id")]
    _id: String,
    owner_id: String,
    suggested_price_cny: i64,
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
            description: "获取当前用户发布的所有商品列表。当用户想查看或管理自己的商品时使用。"
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let owner_id = self
            .ctx
            .current_user_id
            .clone()
            .ok_or_else(|| ToolError("请先登录再进行操作".to_string()))?;

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

// ---------------------------------------------------------------------------
// Unit tests (no DB required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_error_display() {
        let err = ToolError("test error message".to_string());
        assert_eq!(err.to_string(), "Tool error: test error message");
    }

    #[test]
    fn test_tool_error_debug() {
        let err = ToolError("debug test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("ToolError"));
        assert!(debug_str.contains("debug test"));
    }

    #[test]
    fn test_create_listing_args_deserialization() {
        let json = r#"{
            "title": "iPhone 13",
            "category": "electronics",
            "brand": "Apple",
            "condition_score": 8,
            "suggested_price_cny": 500000,
            "defects": ["Minor scratch"],
            "original_description": "Barely used"
        }"#;
        let args: CreateListingArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.title, "iPhone 13");
        assert_eq!(args.category, "electronics");
        assert_eq!(args.brand, "Apple");
        assert_eq!(args.condition_score, 8);
        assert_eq!(args.suggested_price_cny, 500000);
        assert!(args.defects.contains(&"Minor scratch".to_string()));
        assert_eq!(args.original_description, "Barely used");
    }

    #[test]
    fn test_create_listing_args_empty_defects() {
        let json = r#"{
            "title": "Book",
            "category": "books",
            "brand": "Publisher",
            "condition_score": 7,
            "suggested_price_cny": 5000,
            "defects": [],
            "original_description": "Like new"
        }"#;
        let args: CreateListingArgs = serde_json::from_str(json).unwrap();
        assert!(args.defects.is_empty());
    }

    #[test]
    fn test_search_inventory_args_partial() {
        // Only keyword provided
        let json = r#"{"keyword": "iphone"}"#;
        let args: SearchInventoryArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.keyword, Some("iphone".to_string()));
        assert_eq!(args.category, None);
        assert_eq!(args.max_price, None);
        assert_eq!(args.min_condition, None);
    }

    #[test]
    fn test_search_inventory_args_all_filters() {
        let json = r#"{
            "keyword": "laptop",
            "category": "electronics",
            "max_price": 500000,
            "min_condition": 7
        }"#;
        let args: SearchInventoryArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.keyword, Some("laptop".to_string()));
        assert_eq!(args.category, Some("electronics".to_string()));
        assert_eq!(args.max_price, Some(500000));
        assert_eq!(args.min_condition, Some(7));
    }

    #[test]
    fn test_search_inventory_args_empty() {
        let json = r#"{}"#;
        let args: SearchInventoryArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.keyword, None);
        assert_eq!(args.category, None);
        assert_eq!(args.max_price, None);
        assert_eq!(args.min_condition, None);
    }

    #[test]
    fn test_get_listing_details_args() {
        let json = r#"{"listing_id": "listing-123"}"#;
        let args: GetListingDetailsArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.listing_id, "listing-123");
    }

    #[test]
    fn test_update_listing_args_partial() {
        // Only new_price provided
        let json = r#"{"listing_id": "listing-456", "new_price": 450000}"#;
        let args: UpdateListingArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.listing_id, "listing-456");
        assert_eq!(args.new_price, Some(450000));
        assert_eq!(args.new_title, None);
        assert_eq!(args.new_description, None);
    }

    #[test]
    fn test_update_listing_args_all_fields() {
        let json = r#"{
            "listing_id": "listing-789",
            "new_price": 400000,
            "new_title": "Updated Title",
            "new_description": "New description"
        }"#;
        let args: UpdateListingArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.listing_id, "listing-789");
        assert_eq!(args.new_price, Some(400000));
        assert_eq!(args.new_title, Some("Updated Title".to_string()));
        assert_eq!(args.new_description, Some("New description".to_string()));
    }

    #[test]
    fn test_delete_listing_args() {
        let json = r#"{"listing_id": "listing-delete-1"}"#;
        let args: DeleteListingArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.listing_id, "listing-delete-1");
    }

    #[test]
    fn test_purchase_item_intent_args() {
        let json = r#"{"listing_id": "listing-buy-1", "offered_price": 450000}"#;
        let args: PurchaseItemIntentArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.listing_id, "listing-buy-1");
        assert_eq!(args.offered_price, 450000);
    }

    #[test]
    fn test_get_my_listings_args_empty() {
        let json = r#"{}"#;
        let args: GetMyListingsArgs = serde_json::from_str(json).unwrap();
        // Empty struct deserializes successfully
        let _ = args;
    }

    #[test]
    fn test_tool_context_clone() {
        // ToolContext is Clone, verify it compiles
        fn assert_clone<T: Clone>() {}
        assert_clone::<ToolContext>();
    }

    #[test]
    fn test_create_listing_tool_clone() {
        // CreateListingTool is Clone, verify it compiles
        fn assert_clone<T: Clone>() {}
        assert_clone::<CreateListingTool>();
    }

    #[test]
    fn test_search_inventory_tool_clone() {
        // SearchInventoryTool is Clone, verify it compiles
        fn assert_clone<T: Clone>() {}
        assert_clone::<SearchInventoryTool>();
    }
}
