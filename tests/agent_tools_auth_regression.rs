use async_trait::async_trait;
use good4ncu::agents::tools::{
    DeleteListingArgs, DeleteListingTool, EmbedUpdater, ToolContext, ToolError, UpdateListingArgs,
    UpdateListingTool,
};
use good4ncu::services::{notification::NotificationService, BusinessEvent};
use good4ncu::test_infra::with_test_pool;
use rig::tool::Tool;
use sqlx::Row;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
struct NoopEmbedUpdater;

#[async_trait(?Send)]
impl EmbedUpdater for NoopEmbedUpdater {
    async fn embed_and_update(
        &self,
        _content: String,
        _listing_id: String,
        _conn: &mut sqlx::PgConnection,
    ) -> Result<(), ToolError> {
        Ok(())
    }
}

fn build_tool_context(db_pool: sqlx::PgPool, current_user_id: Option<&str>) -> ToolContext {
    let embed_and_insert = Arc::new(
        |_content: String,
         _listing_id: String|
         -> Pin<Box<dyn Future<Output = Result<(), ToolError>> + Send>> {
            Box::pin(async { Ok(()) })
        },
    );
    let (event_tx, _event_rx) = mpsc::channel::<BusinessEvent>(16);

    ToolContext {
        db_pool: db_pool.clone(),
        embed_and_insert,
        embed_updater: Arc::new(NoopEmbedUpdater),
        event_tx,
        current_user_id: current_user_id.map(ToString::to_string),
        notification: NotificationService::new(db_pool),
    }
}

#[tokio::test]
async fn test_update_listing_tool_denies_cross_owner_mutation() {
    with_test_pool(|pool| async move {
        let suffix = Uuid::new_v4().to_string();
        let owner_id = format!("owner-user-{suffix}");
        let attacker_id = format!("attacker-user-{suffix}");
        let listing_id = format!("listing-auth-{suffix}");
        let owner_username = format!("owner-{suffix}");
        let attacker_username = format!("attacker-{suffix}");

        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, 'hash')")
            .bind(&owner_id)
            .bind(&owner_username)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, 'hash')")
            .bind(&attacker_id)
            .bind(&attacker_username)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO inventory (id, title, category, brand, condition_score, suggested_price_cny, defects, owner_id) \
             VALUES ($1, 'Owner Item', 'misc', 'Brand', 8, 10000, '[]', $2)",
        )
        .bind(&listing_id)
        .bind(&owner_id)
        .execute(&pool)
        .await
        .unwrap();

        let tool = UpdateListingTool {
            ctx: build_tool_context(pool.clone(), Some(attacker_id.as_str())),
        };
        let result = tool
            .call(UpdateListingArgs {
                listing_id: listing_id.clone(),
                new_price: Some(9999),
                new_title: None,
                new_description: None,
            })
            .await
            .unwrap();

        assert!(result.contains("or you don't own it"));

        let row = sqlx::query("SELECT suggested_price_cny FROM inventory WHERE id = $1")
            .bind(&listing_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let price: i32 = row.get("suggested_price_cny");
        assert_eq!(price, 10000);
    })
    .await;
}

#[tokio::test]
async fn test_delete_listing_tool_denies_cross_owner_mutation() {
    with_test_pool(|pool| async move {
        let suffix = Uuid::new_v4().to_string();
        let owner_id = format!("owner-user-{suffix}");
        let attacker_id = format!("attacker-user-{suffix}");
        let listing_id = format!("listing-auth-{suffix}");
        let owner_username = format!("owner-{suffix}");
        let attacker_username = format!("attacker-{suffix}");

        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, 'hash')")
            .bind(&owner_id)
            .bind(&owner_username)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, 'hash')")
            .bind(&attacker_id)
            .bind(&attacker_username)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO inventory (id, title, category, brand, condition_score, suggested_price_cny, defects, owner_id) \
             VALUES ($1, 'Owner Item', 'misc', 'Brand', 8, 10000, '[]', $2)",
        )
        .bind(&listing_id)
        .bind(&owner_id)
        .execute(&pool)
        .await
        .unwrap();

        let tool = DeleteListingTool {
            ctx: build_tool_context(pool.clone(), Some(attacker_id.as_str())),
        };
        let result = tool
            .call(DeleteListingArgs {
                listing_id: listing_id.clone(),
            })
            .await
            .unwrap();

        assert!(result.contains("or you don't own it"));

        let row = sqlx::query("SELECT status FROM inventory WHERE id = $1")
            .bind(&listing_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let status: String = row.get("status");
        assert_eq!(status, "active");
    })
    .await;
}

#[tokio::test]
async fn test_mutation_tools_require_authenticated_user() {
    with_test_pool(|pool| async move {
        let update_tool = UpdateListingTool {
            ctx: build_tool_context(pool.clone(), None),
        };
        let update_err = update_tool
            .call(UpdateListingArgs {
                listing_id: "any-listing".to_string(),
                new_price: Some(5000),
                new_title: None,
                new_description: None,
            })
            .await
            .unwrap_err();

        let delete_tool = DeleteListingTool {
            ctx: build_tool_context(pool, None),
        };
        let delete_err = delete_tool
            .call(DeleteListingArgs {
                listing_id: "any-listing".to_string(),
            })
            .await
            .unwrap_err();

        assert!(update_err.to_string().contains("请先登录再进行操作"));
        assert!(delete_err.to_string().contains("请先登录再进行操作"));
    })
    .await;
}
