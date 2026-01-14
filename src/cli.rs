//! CLI argument parsing with subcommands.

use clap::{Parser, Subcommand};

/// Claude Code hook & Telegram Bot integration.
#[derive(Parser)]
#[command(name = "claude-code-telegram")]
#[command(about = "Claude Code hook & Telegram Bot integration")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Handle PermissionRequest hooks (reads from stdin)
    Hook,

    /// Handle Stop hooks for job completion notifications (reads from stdin)
    Stop,

    /// Run the Telegram bot for /start, /help, /status commands
    Bot,
}
