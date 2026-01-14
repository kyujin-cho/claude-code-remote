//! Permission request handler for Claude Code hooks.
//!
//! Handles PermissionRequest hook events by sending Telegram notifications
//! with inline keyboards and waiting for user decisions.

use crate::always_allow::AlwaysAllowManager;
use crate::config::Config;
use crate::error::HookError;
use crate::telegram::{create_permission_keyboard, escape_markdown, parse_callback_data, Decision};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Read};
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{ParseMode, UpdateKind};
use tokio::time::{interval, timeout};

/// Claude Code hook input for permission requests.
#[derive(Debug, Deserialize)]
pub struct HookInput {
    #[serde(default = "default_tool_name")]
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: Value,
}

fn default_tool_name() -> String {
    "unknown".to_string()
}

/// Permission request with a unique ID.
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub tool_name: String,
    pub tool_input: Value,
    pub request_id: String,
}

impl PermissionRequest {
    /// Create a new permission request from hook input.
    pub fn from_hook_input(input: HookInput) -> Self {
        let request_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        Self {
            tool_name: input.tool_name,
            tool_input: input.tool_input,
            request_id,
        }
    }

    /// Format the permission request as a Telegram message.
    pub fn format_message(&self, hostname: Option<&str>) -> String {
        let mut lines = vec![format!(
            "🔐 *Permission Request* `\\[{}\\]`",
            escape_markdown(&self.request_id)
        )];

        if let Some(host) = hostname {
            lines.push(format!("🖥️ *Host:* `{}`", escape_markdown(host)));
        }

        lines.push(String::new());
        lines.push(format!("*Tool:* `{}`", escape_markdown(&self.tool_name)));

        match self.tool_name.as_str() {
            "Bash" => {
                if let Some(command) = self.tool_input.get("command").and_then(|v| v.as_str()) {
                    lines.push(format!(
                        "*Command:*\n```\n{}\n```",
                        escape_markdown(command)
                    ));
                }
            }
            "Edit" | "Write" => {
                if let Some(file_path) = self.tool_input.get("file_path").and_then(|v| v.as_str()) {
                    lines.push(format!("*File:* `{}`", escape_markdown(file_path)));
                }

                if self.tool_name == "Edit" {
                    if let Some(old_string) =
                        self.tool_input.get("old_string").and_then(|v| v.as_str())
                    {
                        let truncated: String = old_string.chars().take(200).collect();
                        lines.push(format!("*Old:*\n```\n{}\n```", escape_markdown(&truncated)));
                    }
                    if let Some(new_string) =
                        self.tool_input.get("new_string").and_then(|v| v.as_str())
                    {
                        let truncated: String = new_string.chars().take(200).collect();
                        lines.push(format!("*New:*\n```\n{}\n```", escape_markdown(&truncated)));
                    }
                }
            }
            _ => {
                let input_str = serde_json::to_string_pretty(&self.tool_input).unwrap_or_default();
                let truncated: String = input_str.chars().take(500).collect();
                lines.push(format!(
                    "*Input:*\n```json\n{}\n```",
                    escape_markdown(&truncated)
                ));
            }
        }

        lines.join("\n")
    }
}

/// Claude Code hook output format.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookOutput {
    pub hook_specific_output: HookSpecificOutput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSpecificOutput {
    pub hook_event_name: String,
    pub decision: DecisionOutput,
}

#[derive(Debug, Serialize)]
pub struct DecisionOutput {
    pub behavior: String,
}

/// Create the hook response JSON.
pub fn create_hook_response(decision: Decision) -> HookOutput {
    HookOutput {
        hook_specific_output: HookSpecificOutput {
            hook_event_name: "PermissionRequest".to_string(),
            decision: DecisionOutput {
                behavior: decision.to_behavior().to_string(),
            },
        },
    }
}

/// Send an auto-approved notification (no buttons).
async fn send_auto_approved_notification(
    bot: &Bot,
    config: &Config,
    request: &PermissionRequest,
) -> Result<(), HookError> {
    let mut lines = vec![
        format!(
            "⚙️ *Auto\\-Approved* `\\[{}\\]`",
            escape_markdown(&request.request_id)
        ),
        format!("🖥️ *Host:* `{}`", escape_markdown(&config.hostname)),
        String::new(),
        format!(
            "*Tool:* `{}` _\\(in always\\-allow list\\)_",
            escape_markdown(&request.tool_name)
        ),
    ];

    match request.tool_name.as_str() {
        "Bash" => {
            if let Some(command) = request.tool_input.get("command").and_then(|v| v.as_str()) {
                lines.push(format!(
                    "*Command:*\n```\n{}\n```",
                    escape_markdown(command)
                ));
            }
        }
        "Edit" | "Write" => {
            if let Some(file_path) = request.tool_input.get("file_path").and_then(|v| v.as_str()) {
                lines.push(format!("*File:* `{}`", escape_markdown(file_path)));
            }
        }
        _ => {
            let input_str = serde_json::to_string_pretty(&request.tool_input).unwrap_or_default();
            let truncated: String = input_str.chars().take(500).collect();
            lines.push(format!(
                "*Input:*\n```json\n{}\n```",
                escape_markdown(&truncated)
            ));
        }
    }

    bot.send_message(config.telegram_chat_id, lines.join("\n"))
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

/// Handle a permission request and wait for user decision.
///
/// Uses manual getUpdates polling to avoid conflicts when multiple
/// permission requests are active simultaneously.
pub async fn handle_permission_request(
    config: &Config,
    always_allow: &AlwaysAllowManager,
    request: &PermissionRequest,
) -> Result<Decision, HookError> {
    let bot = Bot::new(&config.telegram_bot_token);

    // Check if tool is in always-allow list
    if always_allow.is_allowed(&request.tool_name) {
        send_auto_approved_notification(&bot, config, request).await?;
        return Ok(Decision::Allow);
    }

    // Send message with inline keyboard
    let keyboard = create_permission_keyboard(&request.request_id, &request.tool_name);
    let original_message = request.format_message(Some(&config.hostname));
    let message = bot
        .send_message(config.telegram_chat_id, &original_message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    let message_id = message.id;
    let chat_id = config.telegram_chat_id;

    // Poll for callback query with timeout
    let poll_result = timeout(
        Duration::from_secs(300),
        poll_for_callback(&bot, &request.request_id, message_id, chat_id),
    )
    .await;

    match poll_result {
        Ok(Ok(callback_decision)) => {
            // Handle always allow
            if callback_decision == Decision::AlwaysAllow {
                let _ = always_allow.add_tool(&request.tool_name);
            }

            // Determine status text
            let status = match callback_decision {
                Decision::Allow => "✅ Approved".to_string(),
                Decision::Deny => "❌ Denied".to_string(),
                Decision::AlwaysAllow => format!(
                    "🔓 Always Allowed \\(`{}` added to list\\)",
                    escape_markdown(&request.tool_name)
                ),
            };

            // Update message with status
            let new_text = format!("{}\n\n*Status:* {}", original_message, status);
            let _ = bot
                .edit_message_text(chat_id, message_id, new_text)
                .parse_mode(ParseMode::MarkdownV2)
                .await;

            // Return allow for AlwaysAllow
            if callback_decision == Decision::AlwaysAllow {
                Ok(Decision::Allow)
            } else {
                Ok(callback_decision)
            }
        }
        Ok(Err(e)) => {
            // Error during polling
            let _ = bot
                .edit_message_text(
                    chat_id,
                    message_id,
                    format!("{}\n\n*Status:* ❌ Error", original_message),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await;
            Err(e)
        }
        Err(_) => {
            // Timeout
            let _ = bot
                .edit_message_text(
                    chat_id,
                    message_id,
                    format!("{}\n\n*Status:* ⏱️ Timeout \\- Denied", original_message),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await;
            Ok(Decision::Deny)
        }
    }
}

/// Poll for callback query matching our request.
///
/// This uses manual getUpdates polling with message_id filtering to avoid
/// conflicts with other concurrent permission requests.
async fn poll_for_callback(
    bot: &Bot,
    request_id: &str,
    message_id: teloxide::types::MessageId,
    chat_id: teloxide::types::ChatId,
) -> Result<Decision, HookError> {
    let mut poll_interval = interval(Duration::from_millis(500));
    let mut offset: Option<i32> = None;

    loop {
        poll_interval.tick().await;

        // Build getUpdates request
        let mut get_updates = bot.get_updates();
        if let Some(off) = offset {
            get_updates = get_updates.offset(off);
        }
        get_updates = get_updates.timeout(5);
        get_updates =
            get_updates.allowed_updates(vec![teloxide::types::AllowedUpdate::CallbackQuery]);

        let updates = match get_updates.await {
            Ok(updates) => updates,
            Err(_) => continue, // Retry on error
        };

        for update in updates {
            // Update offset for next poll
            offset = Some((update.id.0 + 1) as i32);

            // Check if this is a callback query
            if let UpdateKind::CallbackQuery(query) = update.kind {
                // Check if callback is for our message
                if let Some(msg) = &query.message {
                    if msg.chat().id != chat_id || msg.id() != message_id {
                        continue; // Not our message
                    }
                } else {
                    continue; // No message info
                }

                // Parse callback data
                if let Some(data) = &query.data {
                    if let Some(callback) = parse_callback_data(data) {
                        if callback.request_id == request_id {
                            // Answer callback query to remove loading state
                            let _ = bot.answer_callback_query(&query.id).await;

                            return Ok(callback.decision);
                        }
                    }
                }
            }
        }
    }
}

/// Read JSON input from stdin.
fn read_stdin() -> Result<String, io::Error> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Main entry point for the hook handler.
pub async fn run() -> Result<(), HookError> {
    // Read and parse input
    let input_str = read_stdin()?;
    let input: HookInput = serde_json::from_str(&input_str)?;

    // Load config
    let config = Config::load(None)?;

    // Create request and handler
    let request = PermissionRequest::from_hook_input(input);
    let always_allow = AlwaysAllowManager::new(None);

    // Get decision
    let decision = handle_permission_request(&config, &always_allow, &request).await?;

    // Output response
    let response = create_hook_response(decision);
    println!("{}", serde_json::to_string(&response)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_request_from_hook_input() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls -la"}),
        };

        let request = PermissionRequest::from_hook_input(input);
        assert_eq!(request.tool_name, "Bash");
        assert_eq!(request.request_id.len(), 8);
    }

    #[test]
    fn test_format_message_bash() {
        let request = PermissionRequest {
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls -la"}),
            request_id: "abc12345".to_string(),
        };

        let message = request.format_message(Some("test-host"));
        assert!(message.contains("Bash"));
        assert!(message.contains("ls \\-la"));
        assert!(message.contains("test\\-host"));
    }

    #[test]
    fn test_format_message_edit() {
        let request = PermissionRequest {
            tool_name: "Edit".to_string(),
            tool_input: serde_json::json!({
                "file_path": "/path/to/file.txt",
                "old_string": "old",
                "new_string": "new"
            }),
            request_id: "abc12345".to_string(),
        };

        let message = request.format_message(None);
        assert!(message.contains("Edit"));
        assert!(message.contains("file\\.txt"));
        assert!(message.contains("old"));
        assert!(message.contains("new"));
    }

    #[test]
    fn test_create_hook_response_allow() {
        let response = create_hook_response(Decision::Allow);
        assert_eq!(response.hook_specific_output.decision.behavior, "allow");
    }

    #[test]
    fn test_create_hook_response_deny() {
        let response = create_hook_response(Decision::Deny);
        assert_eq!(response.hook_specific_output.decision.behavior, "deny");
    }
}
