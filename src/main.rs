//! Claude Code messaging integration - CLI entry point.
//!
//! Provides subcommands for hook handlers, Telegram bot, and Signal linking.

mod always_allow;
mod bot;
mod cli;
mod config;
mod error;
mod hook_handler;
mod messenger;
mod stop_handler;
mod telegram;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

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
        #[cfg(feature = "signal")]
        Commands::SignalLink {
            device_name,
            data_path,
        } => {
            let data_path = data_path.unwrap_or_else(config::default_signal_data_path);

            // Ensure data directory exists
            std::fs::create_dir_all(&data_path)
                .context("Failed to create Signal data directory")?;

            println!("📱 Linking device as '{}'...", device_name);
            println!("📂 Data path: {}", data_path.display());

            messenger::signal::link_device(&data_path, &device_name)
                .await
                .context("Failed to link Signal device")?;

            println!("\n✅ Signal device linked successfully!");
            println!("You can now use Signal for permission requests.");
        }
        Commands::Status => {
            print_status().await?;
        }
    }

    Ok(())
}

/// Print configuration status.
async fn print_status() -> Result<()> {
    println!("📊 Claude Code Messaging Status\n");

    // Try to load config
    match Config::load(None) {
        Ok(config) => {
            println!("✅ Configuration: Found");
            println!("   Hostname: {}", config.hostname);
            println!("   Timeout: {}s", config.timeout_seconds);
            println!("   Primary: {}", config.primary_messenger);
            println!();
            println!("📱 Telegram:");
            if let Some(telegram) = &config.telegram {
                println!("   Status: Configured");
                println!("   Chat ID: {}", telegram.chat_id);
            } else {
                println!("   Status: Not configured");
            }

            #[cfg(feature = "signal")]
            {
                println!();
                println!("📱 Signal:");
                if let Some(signal) = &config.signal {
                    println!(
                        "   Status: {}",
                        if signal.enabled {
                            "Enabled"
                        } else {
                            "Disabled"
                        }
                    );
                    println!("   Phone: {}", signal.phone_number);
                    println!("   Device: {}", signal.device_name);
                    println!("   Data: {}", signal.data_path.display());
                } else {
                    println!("   Status: Not configured");
                    println!("   Run 'signal-link' to set up Signal integration");
                }
            }

            #[cfg(not(feature = "signal"))]
            {
                println!();
                println!("📱 Signal: Not available (compile with --features signal)");
            }
        }
        Err(e) => {
            println!("❌ Configuration: Not found or invalid");
            println!("   Error: {}", e);
            println!();
            println!("Create config at ~/.claude/telegram_hook.json:");
            println!(r#"  {{"telegram_bot_token": "...", "telegram_chat_id": "..."}}"#);
        }
    }

    Ok(())
}
