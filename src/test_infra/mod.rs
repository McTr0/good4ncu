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

/// Creates a new test pool connected to the test database.
/// The pool is configured with a short timeout and minimal connections.
async fn create_test_pool() -> PgPool {
    let database_url = db_safety::resolve_test_database_url();

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
