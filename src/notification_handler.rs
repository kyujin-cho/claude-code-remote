//! Notification handler for Claude Code notification hooks.
//!
//! Handles Notification hook events by relaying them to configured messengers.
//! Supports permission prompts, idle prompts, and custom notifications.

use crate::config::Config;
use crate::error::HookError;
use crate::messenger::telegram::TelegramMessenger;
use crate::messenger::Messenger;
use serde::Deserialize;
use std::io::{self, Read};

#[cfg(feature = "discord")]
use crate::messenger::discord::DiscordMessenger;

/// Claude Code notification hook input.
#[derive(Debug, Deserialize)]
pub struct NotificationInput {
    /// Type of notification (e.g., "permission_prompt", "idle_prompt")
    #[serde(default)]
    pub notification_type: String,
    /// Notification message content
    #[serde(default)]
    pub message: String,
    /// Session ID
    #[serde(default)]
    #[allow(dead_code)]
    pub session_id: String,
    /// Current working directory
    #[serde(default)]
    pub cwd: String,
}

/// Format notification for messaging.
fn format_notification(input: &NotificationInput, hostname: &str) -> String {
    let icon = match input.notification_type.as_str() {
        "permission_prompt" => "🔐",
        "idle_prompt" => "💤",
        _ => "📢",
    };

    let type_label = match input.notification_type.as_str() {
        "permission_prompt" => "Permission Required",
        "idle_prompt" => "Idle - Waiting for Input",
        _ => "Notification",
    };

    let mut lines = vec![
        format!("{} **{}**", icon, type_label),
        format!("🖥️ **Host:** {}", hostname),
    ];

    if !input.cwd.is_empty() {
        // Extract project name from cwd
        let project = input
            .cwd
            .split('/')
            .next_back()
            .unwrap_or(&input.cwd);
        lines.push(format!("📁 **Project:** {}", project));
    }

    if !input.message.is_empty() {
        lines.push(String::new());
        // Truncate long messages
        let truncated: String = input.message.chars().take(500).collect();
        if input.message.len() > 500 {
            lines.push(format!("{}...", truncated));
        } else {
            lines.push(truncated);
        }
    }

    lines.join("\n")
}

/// Send notification via the configured messenger.
pub async fn send_notification(config: &Config, input: &NotificationInput) -> Result<(), HookError> {
    let text = format_notification(input, &config.hostname);

    // Try Discord if configured as primary
    #[cfg(feature = "discord")]
    if config.primary_messenger == "discord" {
        if let Some(ref discord_config) = config.discord {
            if discord_config.enabled {
                let messenger =
                    DiscordMessenger::new(&discord_config.bot_token, discord_config.user_id);
                return messenger.send_notification(&text).await;
            }
        }
    }

    // Try Telegram if configured
    if let Some(ref telegram_config) = config.telegram {
        let messenger = TelegramMessenger::new(&telegram_config.bot_token, telegram_config.chat_id);
        return messenger.send_notification(&text).await;
    }

    // Try Discord as fallback
    #[cfg(feature = "discord")]
    if let Some(ref discord_config) = config.discord {
        if discord_config.enabled {
            let messenger =
                DiscordMessenger::new(&discord_config.bot_token, discord_config.user_id);
            return messenger.send_notification(&text).await;
        }
    }

    // No messenger available - silently skip
    Ok(())
}

/// Read JSON input from stdin.
fn read_stdin() -> Result<String, io::Error> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Main entry point for the notification handler.
pub async fn run() -> Result<(), HookError> {
    let input_str = read_stdin()?;
    let input: NotificationInput = serde_json::from_str(&input_str)?;

    let config = Config::load(None)?;

    send_notification(&config, &input).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_notification_permission() {
        let input = NotificationInput {
            notification_type: "permission_prompt".to_string(),
            message: "Claude needs permission to run bash".to_string(),
            session_id: "test123".to_string(),
            cwd: "/home/user/project".to_string(),
        };

        let result = format_notification(&input, "test-host");
        assert!(result.contains("Permission Required"));
        assert!(result.contains("test-host"));
        assert!(result.contains("project"));
    }

    #[test]
    fn test_format_notification_idle() {
        let input = NotificationInput {
            notification_type: "idle_prompt".to_string(),
            message: "Waiting for input".to_string(),
            session_id: "test123".to_string(),
            cwd: "/home/user/myapp".to_string(),
        };

        let result = format_notification(&input, "my-machine");
        assert!(result.contains("Idle"));
        assert!(result.contains("my-machine"));
    }
}
