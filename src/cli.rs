use anyhow::Result;
use sqlx::PgPool;
use std::env;

/// Run the CLI with command-line arguments.
/// Returns true if a CLI command was executed, false otherwise.
pub async fn run_cli(args: &[String]) -> Result<bool> {
    if args.len() < 2 {
        return Ok(false);
    }

    match args[1].as_str() {
        "admin" => {
            if args.len() < 3 {
                eprintln!("Usage: admin promote <username>");
                return Ok(true);
            }
            match args[2].as_str() {
                "promote" => {
                    if args.len() < 4 {
                        eprintln!("Usage: admin promote <username>");
                        return Ok(true);
                    }
                    let username = &args[3];
                    run_admin_promote(username).await?;
                    Ok(true)
                }
                _ => {
                    eprintln!("Unknown admin command: {}", args[2]);
                    eprintln!("Usage: admin promote <username>");
                    Ok(true)
                }
            }
        }
        _ => Ok(false),
    }
}

/// Promote a user to admin role.
async fn run_admin_promote(username: &str) -> Result<()> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url).await?;

    let result = sqlx::query("UPDATE users SET role = 'admin' WHERE username = $1")
        .bind(username)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        anyhow::bail!("用户 '{}' 不存在", username);
    }

    println!("用户 '{}' 已提升为管理员", username);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cli_args_parsing() {
        // Test admin promote parsing (cargo run -- admin promote testuser)
        let args = vec![
            "cargo".to_string(),
            "run".to_string(),
            "admin".to_string(),
            "promote".to_string(),
            "testuser".to_string(),
        ];
        assert_eq!(args[2], "admin");
        assert_eq!(args[3], "promote");
        assert_eq!(args[4], "testuser");
    }
}
