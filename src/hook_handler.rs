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
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::sync::oneshot;
use tokio::time::timeout;

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
                    if let Some(old_string) = self.tool_input.get("old_string").and_then(|v| v.as_str()) {
                        let truncated: String = old_string.chars().take(200).collect();
                        lines.push(format!(
                            "*Old:*\n```\n{}\n```",
                            escape_markdown(&truncated)
                        ));
                    }
                    if let Some(new_string) = self.tool_input.get("new_string").and_then(|v| v.as_str()) {
                        let truncated: String = new_string.chars().take(200).collect();
                        lines.push(format!(
                            "*New:*\n```\n{}\n```",
                            escape_markdown(&truncated)
                        ));
                    }
                }
            }
            _ => {
                let input_str = serde_json::to_string_pretty(&self.tool_input)
                    .unwrap_or_default();
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
            let input_str = serde_json::to_string_pretty(&request.tool_input)
                .unwrap_or_default();
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

    // Create channel for decision signaling
    let (tx, rx) = oneshot::channel::<Decision>();
    let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));

    // Send message with inline keyboard
    let keyboard = create_permission_keyboard(&request.request_id, &request.tool_name);
    let message = bot
        .send_message(config.telegram_chat_id, request.format_message(Some(&config.hostname)))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    // Clone values for the callback handler
    let request_id = request.request_id.clone();
    let tool_name = request.tool_name.clone();
    let always_allow_clone = always_allow.clone();
    let tx_clone = Arc::clone(&tx);
    let bot_clone = bot.clone();
    let chat_id = config.telegram_chat_id;
    let hostname = config.hostname.clone();
    let original_message = request.format_message(Some(&hostname));

    // Spawn callback query handler
    let handler = tokio::spawn(async move {
        let handler = Update::filter_callback_query().endpoint(
            move |bot: Bot, q: CallbackQuery| {
                let request_id = request_id.clone();
                let tool_name = tool_name.clone();
                let always_allow = always_allow_clone.clone();
                let tx = Arc::clone(&tx_clone);
                let original_message = original_message.clone();

                async move {
                    if let Some(data) = &q.data {
                        if let Some(callback) = parse_callback_data(data) {
                            if callback.request_id == request_id {
                                // Handle always allow
                                if callback.decision == Decision::AlwaysAllow {
                                    if let Some(tool) = &callback.tool_name {
                                        let _ = always_allow.add_tool(tool);
                                    }
                                }

                                // Determine status text
                                let status = match callback.decision {
                                    Decision::Allow => "✅ Approved",
                                    Decision::Deny => "❌ Denied",
                                    Decision::AlwaysAllow => {
                                        &format!("🔓 Always Allowed \\(`{}` added to list\\)",
                                            escape_markdown(&tool_name))
                                    }
                                };

                                // Update message
                                if let Some(msg) = q.message {
                                    let new_text = format!(
                                        "{}\n\n*Status:* {}",
                                        original_message,
                                        status
                                    );
                                    let _ = bot
                                        .edit_message_text(msg.chat().id, msg.id(), new_text)
                                        .parse_mode(ParseMode::MarkdownV2)
                                        .await;
                                }

                                // Answer callback query
                                let _ = bot.answer_callback_query(&q.id).await;

                                // Send decision
                                if let Some(sender) = tx.lock().await.take() {
                                    let decision = if callback.decision == Decision::AlwaysAllow {
                                        Decision::Allow
                                    } else {
                                        callback.decision
                                    };
                                    let _ = sender.send(decision);
                                }
                            }
                        }
                    }
                    Ok::<_, teloxide::RequestError>(())
                }
            },
        );

        Dispatcher::builder(bot_clone, handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    });

    // Wait for decision with timeout
    let result = timeout(Duration::from_secs(300), rx).await;

    // Stop the dispatcher
    handler.abort();

    match result {
        Ok(Ok(decision)) => Ok(decision),
        Ok(Err(_)) => {
            // Channel closed without decision
            // Update message to show timeout
            let _ = bot
                .edit_message_text(
                    chat_id,
                    message.id,
                    format!(
                        "{}\n\n*Status:* ⏱️ Timeout \\- Denied",
                        request.format_message(Some(&hostname))
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await;
            Ok(Decision::Deny)
        }
        Err(_) => {
            // Timeout
            let _ = bot
                .edit_message_text(
                    chat_id,
                    message.id,
                    format!(
                        "{}\n\n*Status:* ⏱️ Timeout \\- Denied",
                        request.format_message(Some(&hostname))
                    ),
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await;
            Ok(Decision::Deny)
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
        assert_eq!(
            response.hook_specific_output.decision.behavior,
            "allow"
        );
    }

    #[test]
    fn test_create_hook_response_deny() {
        let response = create_hook_response(Decision::Deny);
        assert_eq!(
            response.hook_specific_output.decision.behavior,
            "deny"
        );
    }
}
