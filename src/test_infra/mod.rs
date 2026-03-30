//! Shared test infrastructure for database integration tests.
//!
//! Design:
//! - Each test gets its own `PgPool` via `PgPool::connect` (one pool per test)
//! - Pool is dropped at end of test, all connections returned
//! - Tests MUST be run with `--test-threads=1` for proper isolation
//! - OR use serial test execution where each test completes before the next starts

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::future::Future;

pub mod db_safety;

pub(crate) async fn ensure_test_database_exists(database_url: &str) -> bool {
    let db_name = db_safety::extract_db_name(database_url)
        .expect("test database URL must include database name");

    if !db_safety::is_safe_db_identifier(&db_name) {
        panic!(
            "Refusing to auto-create test database with unsafe identifier '{}': only [A-Za-z0-9_] allowed",
            db_name
        );
    }

    // Fast path for environments where the test DB already exists and is reachable,
    // even if the role cannot connect to the admin database.
    match PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await
    {
        Ok(pool) => {
            drop(pool);
            return false;
        }
        Err(e) => {
            let is_missing_database = e
                .as_database_error()
                .and_then(|db_err| db_err.code())
                .as_deref()
                == Some("3D000");
            if !is_missing_database {
                panic!("Failed to connect to test database '{db_name}': {e}");
            }
        }
    }

    let admin_url = db_safety::with_database_name(database_url, "postgres")
        .expect("failed to construct admin database URL");

    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&admin_url)
        .await
        .expect("Failed to connect to postgres admin database");

    let quoted_db_name = format!("\"{}\"", db_name);
    let create_sql = format!("CREATE DATABASE {quoted_db_name}");

    match sqlx::query(&create_sql).execute(&admin_pool).await {
        Ok(_) => true,
        Err(e) => {
            let is_duplicate_database = e
                .as_database_error()
                .and_then(|db_err| db_err.code())
                .as_deref()
                == Some("42P04");
            if is_duplicate_database {
                return false;
            }

            // If create failed for a non-duplicate reason, make one final attempt to connect
            // in case another process created the DB concurrently.
            match PgPoolOptions::new()
                .max_connections(1)
                .min_connections(0)
                .acquire_timeout(std::time::Duration::from_secs(5))
                .connect(database_url)
                .await
            {
                Ok(pool) => {
                    drop(pool);
                    false
                }
                Err(connect_err) => {
                    panic!(
                        "Failed to auto-create test database '{db_name}': {e}; and follow-up connect failed: {connect_err}"
                    );
                }
            }
        }
    }
}

pub(crate) async fn ensure_test_schema_ready(database_url: &str) {
    fn migration_error_code(err: &sqlx::migrate::MigrateError) -> Option<String> {
        match err {
            sqlx::migrate::MigrateError::ExecuteMigration(sqlx::Error::Database(db_err), _) => {
                db_err.code().map(|c| c.into_owned())
            }
            _ => None,
        }
    }

    fn is_duplicate_schema_conflict(err: &sqlx::migrate::MigrateError) -> bool {
        matches!(migration_error_code(err).as_deref(), Some("42701") | Some("42P07"))
    }

    async fn reset_public_schema(pool: &PgPool) {
        sqlx::query("DROP SCHEMA IF EXISTS public CASCADE")
            .execute(pool)
            .await
            .expect("Failed to drop legacy public schema");
        sqlx::query("CREATE SCHEMA public")
            .execute(pool)
            .await
            .expect("Failed to recreate public schema");
    }

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await
        .expect("Failed to connect to test database for migration");

    sqlx::query("SELECT pg_advisory_lock(hashtext('good4ncu_test_schema_migrate')::bigint)")
        .execute(&pool)
        .await
        .expect("Failed to acquire migration advisory lock");

    let has_sqlx_migrations: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = '_sqlx_migrations')",
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to inspect sqlx migrations table");

    let has_users_table: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'users')",
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to inspect users table");

    if !has_sqlx_migrations && has_users_table {
        tracing::warn!("Detected legacy test DB without migration ledger; resetting schema");
        reset_public_schema(&pool).await;
    }

    let migrate_result = match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(()) => Ok(()),
        Err(e) if is_duplicate_schema_conflict(&e) => {
            tracing::warn!(
                code = ?migration_error_code(&e),
                "Duplicate schema conflict during migration; resetting schema and retrying"
            );
            reset_public_schema(&pool).await;
            sqlx::migrate!("./migrations").run(&pool).await
        }
        Err(e) => Err(e),
    };

    sqlx::query("SELECT pg_advisory_unlock(hashtext('good4ncu_test_schema_migrate')::bigint)")
        .execute(&pool)
        .await
        .expect("Failed to release migration advisory lock");

    migrate_result.expect("Failed to apply migrations to test database");
}

/// Creates a new test pool connected to the test database.
/// The pool is configured with a short timeout and minimal connections.
async fn create_test_pool() -> PgPool {
    let database_url = db_safety::resolve_test_database_url();

    let _created = ensure_test_database_exists(&database_url).await;
    ensure_test_schema_ready(&database_url).await;

    PgPoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Runs `test_body` with a fresh pool. The pool has `max_connections = 1`
/// so all operations serialize on a single connection.
/// After `test_body` completes, the pool is dropped and all connections returned.
pub async fn with_test_pool<F, Fut>(test_body: F)
where
    F: FnOnce(PgPool) -> Fut,
    Fut: Future<Output = ()>,
{
    let pool = create_test_pool().await;

    // Clean all tables before test runs.
    // This runs on pool's single connection before test body starts.
    let clean_tables = [
        "chat_messages",
        "hitl_requests",
        "notifications",
        "watchlist",
        "chat_connections",
        "orders",
        "inventory",
        "documents",
        "refresh_tokens",
        "users",
    ];
    for table in &clean_tables {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&pool)
            .await
            .expect("DELETE must succeed");
    }

    test_body(pool).await;

    // pool is dropped here, returning all connections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_test_database_url_prefers_test_name() {
        std::env::set_var("TEST_DATABASE_URL", "postgres://localhost/good4ncu_test");
        let resolved = db_safety::resolve_test_database_url();
        assert!(resolved.contains("good4ncu_test"));
        std::env::remove_var("TEST_DATABASE_URL");
    }

    #[test]
    fn test_reject_unsafe_db_identifier_for_auto_create() {
        assert!(!db_safety::is_safe_db_identifier("bad-db"));
    }

    #[tokio::test]
    async fn test_with_test_pool_smoke() {
        with_test_pool(|_pool| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        })
        .await;
    }

    #[tokio::test]
    async fn test_with_test_pool_insert_and_verify() {
        with_test_pool(|pool| async move {
            let rows = sqlx::query("SELECT COUNT(*) as c FROM users")
                .fetch_one(&pool)
                .await
                .unwrap();
            let count: i64 = sqlx::Row::get(&rows, "c");
            assert_eq!(count, 0, "users table should be empty after clean");
        })
        .await;
    }
}
