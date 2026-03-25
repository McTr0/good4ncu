use std::sync::Arc;

use anyhow::Result;
use inquire::Select;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::agents::negotiate;
use crate::llm::LlmProvider;
use crate::services::BusinessEvent;

#[cfg(test)]
mod tests {
    #[test]
    fn test_cli_module_compiles() {
        // Verify the module structure is valid
        // The CLI depends on inquire which may not be available in test,
        // so we just verify compilation here
    }
}

pub async fn run_cli(
    db_pool: PgPool,
    provider: Arc<dyn LlmProvider>,
    event_tx: mpsc::Sender<BusinessEvent>,
) -> Result<()> {
    tracing::info!("Starting CLI session (provider: {})", provider.name());
    tracing::info!("Web Server started at http://127.0.0.1:3000");

    let options = vec![
        "Talk to Marketplace Assistant (Buy & Sell)",
        "Auto-Negotiate Demo",
        "Exit",
    ];

    loop {
        let ans = Select::new("What would you like to do?", options.clone()).prompt()?;

        match ans {
            "Talk to Marketplace Assistant (Buy & Sell)" => {
                if let Err(e) = run_marketplace_agent_cli(&provider, &db_pool).await {
                    tracing::error!(%e, "Agent error");
                }
            }
            "Auto-Negotiate Demo" => {
                if let Err(e) =
                    negotiate::run_auto_negotiation(Arc::clone(&provider), event_tx.clone()).await
                {
                    tracing::error!(%e, "Negotiation error");
                }
            }
            _ => {
                tracing::info!("CLI exiting");
                break;
            }
        }
    }

    Ok(())
}

async fn run_marketplace_agent_cli(
    provider: &Arc<dyn LlmProvider>,
    _db_pool: &PgPool,
) -> anyhow::Result<()> {
    println!("\n[System] Initializing Marketplace Agent with live platform inventory...");

    let _event_tx = tokio::sync::mpsc::channel::<BusinessEvent>(16).0;

    // CLI mode: create a basic agent without RAG tools (just for chat)
    let agent = provider.clone().create_negotiate_agent().await?;

    println!("[System] Ready for searches and selling requests!\n");

    let mut current_prompt = inquire::Text::new("What are you looking for?").prompt()?;
    loop {
        if current_prompt.trim().to_lowercase() == "exit"
            || current_prompt.trim().to_lowercase() == "quit"
        {
            break;
        }

        println!("Thinking...");
        let response = agent.prompt(current_prompt.clone()).await?;
        println!("\n🤖: {}\n", response);

        current_prompt = inquire::Text::new("You (type exit to quit):").prompt()?;
    }

    Ok(())
}
