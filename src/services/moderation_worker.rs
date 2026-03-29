//! Background worker for async image moderation.
//!
//! Polls the `moderation_jobs` table for pending jobs and calls the external
//! image moderation API (Alibaba IMAN). Updates job status to approved/rejected/failed.

use sqlx::PgPool;
use std::time::Duration;

/// Polling interval between batch scans.
const POLL_INTERVAL_SECS: u64 = 5;

/// Maximum jobs to process per poll cycle.
const MAX_JOBS_PER_CYCLE: i64 = 20;

/// Maximum retry attempts before marking a job as failed.
const MAX_RETRIES: i32 = 3;

/// Run the moderation worker loop.
/// Spawn this as a background `tokio::spawn` task in `main.rs`.
pub async fn run_moderation_worker(db: PgPool) {
    tracing::info!("Moderation worker started");
    loop {
        match process_pending_jobs(&db).await {
            Ok(count) => {
                if count > 0 {
                    tracing::debug!(count, "moderation jobs processed");
                }
            }
            Err(e) => {
                tracing::error!(%e, "moderation worker error");
            }
        }
        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

/// Fetch and process up to MAX_JOBS_PER_CYCLE pending jobs.
async fn process_pending_jobs(db: &PgPool) -> anyhow::Result<i64> {
    // Claim jobs by updating status from 'pending' → 'processing' atomically.
    // This prevents multiple workers from claiming the same job.
    let rows = sqlx::query_as::<_, (String, String, String, String, i32)>(
        r#"
        WITH claimed AS (
            SELECT id, resource_type, resource_id, image_url, retry_count
            FROM moderation_jobs
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE moderation_jobs m
        SET status = 'processing'
        FROM claimed c
        WHERE m.id = c.id
        RETURNING m.id, c.resource_type, c.resource_id, c.image_url, c.retry_count
        "#,
    )
    .bind(MAX_JOBS_PER_CYCLE)
    .fetch_all(db)
    .await?;

    if rows.is_empty() {
        return Ok(0);
    }

    let count = rows.len() as i64;
    for (id, resource_type, resource_id, image_url, retry_count) in rows {
        let result = moderate_image(&image_url).await;
        let (new_status, reject_reason) = match result {
            Ok(true) => ("approved", None),
            Ok(false) => ("rejected", Some("图片内容不合规".to_string())),
            Err(e) => {
                tracing::warn!(job_id = %id, %e, "moderation API call failed");
                if retry_count + 1 >= MAX_RETRIES {
                    ("failed", Some(format!("审核服务错误: {}", e)))
                } else {
                    // Increment retry count and put back to pending.
                    sqlx::query(
                        "UPDATE moderation_jobs SET status = 'pending', retry_count = retry_count + 1 WHERE id = $1",
                    )
                    .bind(&id)
                    .execute(db)
                    .await
                    .ok();
                    continue;
                }
            }
        };

        sqlx::query(
            r#"
            UPDATE moderation_jobs
            SET status = $1, reject_reason = $2, processed_at = CURRENT_TIMESTAMP
            WHERE id = $3
            "#,
        )
        .bind(new_status)
        .bind(&reject_reason)
        .bind(&id)
        .execute(db)
        .await
        .ok();

        // Update per-resource moderation status.
        update_resource_status(db, &resource_type, &resource_id, new_status)
            .await
            .ok();
    }

    Ok(count)
}

/// Call the external image moderation API for the given URL.
/// Returns `Ok(true)` = approved, `Ok(false)` = rejected, `Err` = API error.
async fn moderate_image(image_url: &str) -> anyhow::Result<bool> {
    // TODO: Integrate with Alibaba Cloud IMAN when OSS credentials are available.
    // For now, simulate approval for all images to unblock development.
    // Real implementation:
    //   POST https://imagerecog.cn-shanghai.aliyuncs.com/v2/openapi/moderation/async
    //   Headers: Authorization: ACOS-V2AccessKeyId:..., X-ACS-Signature:...
    //   Body: { "tasks": [{ "image_url": image_url }], "async": true }
    //
    // For production, replace this stub with actual IMAN API call.
    tracing::debug!(url = %image_url, "image moderation (stub — always approved)");
    Ok(true)
}

/// Update the per-resource moderation status column.
async fn update_resource_status(
    db: &PgPool,
    resource_type: &str,
    resource_id: &str,
    status: &str,
) -> anyhow::Result<()> {
    match resource_type {
        "listing_image" => {
            sqlx::query("UPDATE inventory SET images_moderation_status = $1 WHERE id = $2")
                .bind(status)
                .bind(resource_id)
                .execute(db)
                .await?;
        }
        "chat_image" => {
            sqlx::query("UPDATE chat_messages SET moderation_status = $1 WHERE id = $2")
                .bind(status)
                .bind(resource_id)
                .execute(db)
                .await?;
        }
        "avatar" => {
            sqlx::query("UPDATE users SET avatar_moderation_status = $1 WHERE id = $2")
                .bind(status)
                .bind(resource_id)
                .execute(db)
                .await?;
        }
        _ => {
            tracing::warn!(resource_type, "unknown resource type in moderation job");
        }
    }
    Ok(())
}
