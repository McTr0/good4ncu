//! Admin service for platform-wide management.

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct AdminService {
    db: PgPool,
}

#[derive(serde::Serialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub admin_id: String,
    pub action: String,
    pub target_id: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub memo: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl AdminService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// List audit logs with pagination.
    pub async fn list_audit_logs(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AuditLogEntry>, i64)> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_audit_logs")
            .fetch_one(&self.db)
            .await?;

        let rows = sqlx::query_as!(
            AuditLogEntry,
            r#"
            SELECT id, admin_id, action, target_id, old_value, new_value, memo, created_at
            FROM admin_audit_logs
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.db)
        .await?;

        Ok((rows, total))
    }

    /// Log an administrative action to the audit trail.
    pub async fn log_action(
        &self,
        admin_id: &str,
        action: &str,
        target_id: Option<&str>,
        old_value: Option<&str>,
        new_value: Option<&str>,
        memo: Option<&str>,
    ) -> Result<()> {
        let audit_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO admin_audit_logs (id, admin_id, action, target_id, old_value, new_value, memo)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&audit_id)
        .bind(admin_id)
        .bind(action)
        .bind(target_id)
        .bind(old_value)
        .bind(new_value)
        .bind(memo)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
