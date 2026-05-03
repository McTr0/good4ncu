use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use sqlx::Row;

/// Initializes the database: creates the pgvector extension and runs migrations.
/// Returns a single PgPool that handles both relational and vector data.
pub async fn init_db(database_url: &str) -> Result<PgPool> {
    // Create a PgPool for relational + vector data
    let db_pool = PgPoolOptions::new()
        .min_connections(2) // Pre-warm pool to reduce cold-start latency
        .max_connections(20)
        .connect(database_url)
        .await?;

    // Enable pgvector extension (creates the vector type and operators)
    // This must be done before running migrations since the vector type is needed
    // by the documents table migration.
    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(&db_pool)
        .await?;

    // Run versioned migrations (includes all CREATE TABLE, CREATE INDEX, etc.)
    // Keep the literal path here so sqlx embeds the current on-disk migration set at compile time, including new files.
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    Ok(db_pool)
}

pub async fn assert_documents_embedding_dim(db_pool: &PgPool, expected_dim: usize) -> Result<()> {
    let row = sqlx::query(
        r#"
        SELECT format_type(a.atttypid, a.atttypmod) AS embedding_type
        FROM pg_attribute a
        JOIN pg_class c ON c.oid = a.attrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = current_schema()
          AND c.relname = 'documents'
          AND a.attname = 'embedding'
          AND a.attnum > 0
          AND NOT a.attisdropped
        "#,
    )
    .fetch_optional(db_pool)
    .await?;

    let row = row.ok_or_else(|| anyhow::anyhow!("documents.embedding column not found"))?;
    let embedding_type: String = row.get("embedding_type");
    let actual_dim = parse_vector_type_dim(&embedding_type).ok_or_else(|| {
        anyhow::anyhow!(
            "failed to parse documents.embedding type '{embedding_type}' as vector(dim)"
        )
    })?;

    if actual_dim != expected_dim {
        anyhow::bail!(
            "documents.embedding dimension mismatch: schema has {actual_dim}, config expects {expected_dim}"
        );
    }

    Ok(())
}

pub async fn assert_uuid_shadow_drift_zero(db_pool: &PgPool) -> Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT relation_name, missing_shadow_ids, fk_drift_rows
        FROM uuid_shadow_divergence
        WHERE missing_shadow_ids > 0 OR fk_drift_rows > 0
        ORDER BY relation_name
        "#,
    )
    .fetch_all(db_pool)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    let details = rows
        .into_iter()
        .map(|row| {
            let relation_name: String = row.get("relation_name");
            let missing_shadow_ids: i64 = row.get("missing_shadow_ids");
            let fk_drift_rows: i64 = row.get("fk_drift_rows");
            format!(
                "{relation_name}(missing_shadow_ids={missing_shadow_ids}, fk_drift_rows={fk_drift_rows})"
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    anyhow::bail!("uuid shadow drift detected: {details}");
}

fn parse_vector_type_dim(vector_type: &str) -> Option<usize> {
    vector_type
        .strip_prefix("vector(")?
        .strip_suffix(')')?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::parse_vector_type_dim;

    #[test]
    fn parse_vector_type_dim_accepts_valid_pgvector_type() {
        assert_eq!(parse_vector_type_dim("vector(768)"), Some(768));
    }

    #[test]
    fn parse_vector_type_dim_rejects_unexpected_shapes() {
        assert_eq!(parse_vector_type_dim("text"), None);
        assert_eq!(parse_vector_type_dim("vector"), None);
        assert_eq!(parse_vector_type_dim("vector(foo)"), None);
    }
}
