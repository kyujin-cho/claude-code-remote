//! Messenger abstraction layer for multi-platform support.
//!
//! Provides a trait-based abstraction over messaging platforms (Telegram, Signal, etc.)
//! to enable sending permission requests and receiving user decisions.

mod types;

pub mod telegram;

#[cfg(feature = "signal")]
pub mod signal;

#[cfg(feature = "discord")]
pub mod discord;

pub use types::{Decision, PermissionMessage};

use crate::config::Config;
use crate::error::HookError;
use async_trait::async_trait;
use std::time::Duration;

/// Abstraction over messaging platforms for permission request handling.
#[async_trait]
pub trait Messenger: Send + Sync {
    /// Send a permission request and wait for user decision.
    ///
    /// # Arguments
    /// * `message` - The permission request details
    /// * `timeout` - Maximum time to wait for a response
    ///
    /// # Returns
    /// The user's decision (Allow, Deny, or AlwaysAllow)
    async fn send_permission_request(
        &self,
        message: &PermissionMessage,
        timeout: Duration,
    ) -> Result<Decision, HookError>;

    /// Send a notification message (no response expected).
    ///
    /// Used for auto-approved notifications and job completion alerts.
    async fn send_notification(&self, text: &str) -> Result<(), HookError>;

    /// Send an auto-approved notification with request details.
    async fn send_auto_approved(&self, message: &PermissionMessage) -> Result<(), HookError>;

    /// Get the platform name for logging purposes.
    #[allow(dead_code)]
    fn platform_name(&self) -> &'static str;
}

/// Resolve the configured messenger based on primary_messenger preference and fallbacks.
///
/// Priority: primary messenger -> telegram -> discord (fallback)
pub fn resolve_messenger(config: &Config) -> Result<Box<dyn Messenger>, HookError> {
    // Try Discord if configured as primary
    #[cfg(feature = "discord")]
    if config.primary_messenger == "discord" {
        if let Some(ref discord_config) = config.discord {
            if discord_config.enabled {
                return Ok(Box::new(discord::DiscordMessenger::new(
                    &discord_config.bot_token,
                    discord_config.user_id,
                )));
            }
        }
    }

    // Try Telegram if configured as primary or as fallback
    if let Some(ref telegram_config) = config.telegram {
        return Ok(Box::new(telegram::TelegramMessenger::new(
            &telegram_config.bot_token,
            telegram_config.chat_id,
        )));
    }

    // Try Discord as fallback if telegram not available
    #[cfg(feature = "discord")]
    if let Some(ref discord_config) = config.discord {
        if discord_config.enabled {
            return Ok(Box::new(discord::DiscordMessenger::new(
                &discord_config.bot_token,
                discord_config.user_id,
            )));
        }
    }

    Err(HookError::ConfigError(
        crate::error::ConfigError::MissingField("no messenger configured".to_string()),
    ))
}
