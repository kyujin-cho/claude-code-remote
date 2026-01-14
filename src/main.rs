//! Claude Code Telegram Bot - CLI entry point.
//!
//! Provides subcommands for different hook handlers and the bot.

mod always_allow;
mod bot;
mod cli;
mod config;
mod error;
mod hook_handler;
mod stop_handler;
mod telegram;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Hook => {
            hook_handler::run()
                .await
                .context("Failed to handle permission request")?;
        }
        Commands::Stop => {
            stop_handler::run()
                .await
                .context("Failed to handle stop event")?;
        }
        Commands::Bot => {
            bot::run().await.context("Failed to run Telegram bot")?;
        }
    }

    Ok(())
}
