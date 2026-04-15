//! Notification handler for Claude Code notification hooks.
//!
//! Handles Notification hook events by relaying them to configured messengers.
//! Supports permission prompts, idle prompts, and custom notifications.

use crate::config::Config;
use crate::error::HookError;
use crate::messenger;
use serde::Deserialize;

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
        let project = input.cwd.split('/').next_back().unwrap_or(&input.cwd);
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
pub async fn send_notification(
    config: &Config,
    input: &NotificationInput,
) -> Result<(), HookError> {
    let text = format_notification(input, &config.hostname);

    let resolved = match messenger::resolve_messenger(config) {
        Ok(m) => m,
        Err(_) => return Ok(()), // No messenger available - silently skip
    };

    resolved.send_notification(&text).await
}

/// Main entry point for the notification handler.
pub async fn run() -> Result<(), HookError> {
    let input_str = crate::util::read_stdin()?;
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
