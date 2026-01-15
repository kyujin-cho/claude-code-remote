//! CLI argument parsing with subcommands.

use clap::{Parser, Subcommand};
#[cfg(feature = "signal")]
use std::path::PathBuf;

/// Claude Code hook & messaging integration.
///
/// Supports Telegram (default), Discord (with --features discord),
/// and Signal (with --features signal).
#[derive(Parser)]
#[command(name = "claude-code-telegram")]
#[command(about = "Claude Code hook & messaging integration (Telegram, Discord, Signal)")]
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

    /// Handle Notification hooks for relaying Claude Code notifications (reads from stdin)
    Notify,

    /// Send a custom message to configured messengers
    Relay {
        /// Message to send
        message: String,
    },

    /// Run the Telegram bot for /start, /help, /status commands
    Bot,

    /// Link as a Signal secondary device (requires --features signal)
    #[cfg(feature = "signal")]
    SignalLink {
        /// Device name to register with Signal
        #[arg(long, default_value = "claude-code-hook")]
        device_name: String,

        /// Path to store Signal protocol data
        #[arg(long)]
        data_path: Option<PathBuf>,
    },

    /// Show current configuration status
    Status,
}
