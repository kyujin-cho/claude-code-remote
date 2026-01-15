//! Telegram messenger implementation.
//!
//! Implements the Messenger trait for Telegram using inline keyboards
//! for permission decisions.

use super::{Decision, Messenger, PermissionMessage};
use crate::error::HookError;
use async_trait::async_trait;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{
    ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode, UpdateKind,
};
use tokio::time::{interval, timeout};

/// Telegram messenger for permission requests.
pub struct TelegramMessenger {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramMessenger {
    /// Create a new Telegram messenger.
    pub fn new(bot_token: &str, chat_id: ChatId) -> Self {
        Self {
            bot: Bot::new(bot_token),
            chat_id,
        }
    }
}

#[async_trait]
impl Messenger for TelegramMessenger {
    async fn send_permission_request(
        &self,
        message: &PermissionMessage,
        request_timeout: Duration,
    ) -> Result<Decision, HookError> {
        // Send message with inline keyboard
        let keyboard = create_permission_keyboard(&message.request_id, &message.tool_name);
        let original_message = format_permission_message(message);
        let sent = self
            .bot
            .send_message(self.chat_id, &original_message)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;

        let message_id = sent.id;

        // Poll for callback query with timeout
        let poll_result = timeout(
            request_timeout,
            poll_for_callback(&self.bot, &message.request_id, message_id, self.chat_id),
        )
        .await;

        match poll_result {
            Ok(Ok(callback_decision)) => {
                // Determine status text
                let status = match callback_decision {
                    Decision::Allow => "✅ Approved".to_string(),
                    Decision::Deny => "❌ Denied".to_string(),
                    Decision::AlwaysAllow => format!(
                        "🔓 Always Allowed \\(`{}` added to list\\)",
                        escape_markdown(&message.tool_name)
                    ),
                };

                // Update message with status
                let new_text = format!("{}\n\n*Status:* {}", original_message, status);
                let _ = self
                    .bot
                    .edit_message_text(self.chat_id, message_id, new_text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;

                Ok(callback_decision)
            }
            Ok(Err(e)) => {
                // Error during polling
                let _ = self
                    .bot
                    .edit_message_text(
                        self.chat_id,
                        message_id,
                        format!("{}\n\n*Status:* ❌ Error", original_message),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
                Err(e)
            }
            Err(_) => {
                // Timeout - deny by default
                let _ = self
                    .bot
                    .edit_message_text(
                        self.chat_id,
                        message_id,
                        format!("{}\n\n*Status:* ⏱️ Timeout \\- Denied", original_message),
                    )
                    .parse_mode(ParseMode::MarkdownV2)
                    .await;
                Ok(Decision::Deny)
            }
        }
    }

    async fn send_notification(&self, text: &str) -> Result<(), HookError> {
        self.bot
            .send_message(self.chat_id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        Ok(())
    }

    async fn send_auto_approved(&self, message: &PermissionMessage) -> Result<(), HookError> {
        let text = format_auto_approved_message(message);
        self.send_notification(&text).await
    }

    fn platform_name(&self) -> &'static str {
        "Telegram"
    }
}

/// Create an inline keyboard for permission requests.
fn create_permission_keyboard(request_id: &str, tool_name: &str) -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![
            InlineKeyboardButton::callback("✅ Allow", format!("{}:allow", request_id)),
            InlineKeyboardButton::callback("❌ Deny", format!("{}:deny", request_id)),
        ],
        vec![InlineKeyboardButton::callback(
            "🔓 Always Allow",
            format!("{}:always_allow:{}", request_id, tool_name),
        )],
    ];

    InlineKeyboardMarkup::new(buttons)
}

/// Parsed callback data from a button press.
#[derive(Debug, Clone)]
struct CallbackData {
    request_id: String,
    decision: Decision,
    #[allow(dead_code)]
    tool_name: Option<String>,
}

/// Parse callback data from a button press.
fn parse_callback_data(data: &str) -> Option<CallbackData> {
    let parts: Vec<&str> = data.split(':').collect();

    if parts.len() < 2 {
        return None;
    }

    let request_id = parts[0].to_string();
    let decision = match parts[1] {
        "allow" => Decision::Allow,
        "deny" => Decision::Deny,
        "always_allow" => Decision::AlwaysAllow,
        _ => return None,
    };

    let tool_name = if parts.len() >= 3 {
        Some(parts[2].to_string())
    } else {
        None
    };

    Some(CallbackData {
        request_id,
        decision,
        tool_name,
    })
}

/// Poll for callback query matching our request.
async fn poll_for_callback(
    bot: &Bot,
    request_id: &str,
    message_id: MessageId,
    chat_id: ChatId,
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

/// Escape special characters for Telegram MarkdownV2 format.
pub fn escape_markdown(text: &str) -> String {
    let special_chars = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
    ];
    let mut result = String::with_capacity(text.len() * 2);

    for c in text.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }

    result
}

/// Format a permission request as a Telegram message.
fn format_permission_message(message: &PermissionMessage) -> String {
    let mut lines = vec![format!(
        "🔐 *Permission Request* `\\[{}\\]`",
        escape_markdown(&message.request_id)
    )];

    lines.push(format!(
        "🖥️ *Host:* `{}`",
        escape_markdown(&message.hostname)
    ));
    lines.push(String::new());
    lines.push(format!("*Tool:* `{}`", escape_markdown(&message.tool_name)));

    match message.tool_name.as_str() {
        "Bash" => {
            if let Some(command) = message.tool_input.get("command").and_then(|v| v.as_str()) {
                lines.push(format!(
                    "*Command:*\n```\n{}\n```",
                    escape_markdown(command)
                ));
            }
        }
        "Edit" | "Write" => {
            if let Some(file_path) = message.tool_input.get("file_path").and_then(|v| v.as_str()) {
                lines.push(format!("*File:* `{}`", escape_markdown(file_path)));
            }

            if message.tool_name == "Edit" {
                if let Some(old_string) = message
                    .tool_input
                    .get("old_string")
                    .and_then(|v| v.as_str())
                {
                    let truncated: String = old_string.chars().take(200).collect();
                    lines.push(format!("*Old:*\n```\n{}\n```", escape_markdown(&truncated)));
                }
                if let Some(new_string) = message
                    .tool_input
                    .get("new_string")
                    .and_then(|v| v.as_str())
                {
                    let truncated: String = new_string.chars().take(200).collect();
                    lines.push(format!("*New:*\n```\n{}\n```", escape_markdown(&truncated)));
                }
            }
        }
        _ => {
            let input_str = serde_json::to_string_pretty(&message.tool_input).unwrap_or_default();
            let truncated: String = input_str.chars().take(500).collect();
            lines.push(format!(
                "*Input:*\n```json\n{}\n```",
                escape_markdown(&truncated)
            ));
        }
    }

    lines.join("\n")
}

/// Format an auto-approved notification.
fn format_auto_approved_message(message: &PermissionMessage) -> String {
    let mut lines = vec![
        format!(
            "⚙️ *Auto\\-Approved* `\\[{}\\]`",
            escape_markdown(&message.request_id)
        ),
        format!("🖥️ *Host:* `{}`", escape_markdown(&message.hostname)),
        String::new(),
        format!(
            "*Tool:* `{}` _\\(in always\\-allow list\\)_",
            escape_markdown(&message.tool_name)
        ),
    ];

    match message.tool_name.as_str() {
        "Bash" => {
            if let Some(command) = message.tool_input.get("command").and_then(|v| v.as_str()) {
                lines.push(format!(
                    "*Command:*\n```\n{}\n```",
                    escape_markdown(command)
                ));
            }
        }
        "Edit" | "Write" => {
            if let Some(file_path) = message.tool_input.get("file_path").and_then(|v| v.as_str()) {
                lines.push(format!("*File:* `{}`", escape_markdown(file_path)));
            }
        }
        _ => {
            let input_str = serde_json::to_string_pretty(&message.tool_input).unwrap_or_default();
            let truncated: String = input_str.chars().take(500).collect();
            lines.push(format!(
                "*Input:*\n```json\n{}\n```",
                escape_markdown(&truncated)
            ));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callback_data_allow() {
        let data = parse_callback_data("abc123:allow").unwrap();
        assert_eq!(data.request_id, "abc123");
        assert_eq!(data.decision, Decision::Allow);
        assert!(data.tool_name.is_none());
    }

    #[test]
    fn test_parse_callback_data_deny() {
        let data = parse_callback_data("abc123:deny").unwrap();
        assert_eq!(data.request_id, "abc123");
        assert_eq!(data.decision, Decision::Deny);
    }

    #[test]
    fn test_parse_callback_data_always_allow() {
        let data = parse_callback_data("abc123:always_allow:Bash").unwrap();
        assert_eq!(data.request_id, "abc123");
        assert_eq!(data.decision, Decision::AlwaysAllow);
        assert_eq!(data.tool_name, Some("Bash".to_string()));
    }

    #[test]
    fn test_parse_callback_data_invalid() {
        assert!(parse_callback_data("invalid").is_none());
        assert!(parse_callback_data("abc123:unknown").is_none());
    }

    #[test]
    fn test_decision_to_behavior() {
        assert_eq!(Decision::Allow.to_behavior(), "allow");
        assert_eq!(Decision::Deny.to_behavior(), "deny");
        assert_eq!(Decision::AlwaysAllow.to_behavior(), "allow");
    }

    #[test]
    fn test_escape_markdown() {
        assert_eq!(escape_markdown("hello"), "hello");
        assert_eq!(escape_markdown("hello_world"), "hello\\_world");
        assert_eq!(escape_markdown("test.txt"), "test\\.txt");
        assert_eq!(escape_markdown("*bold*"), "\\*bold\\*");
    }

    #[test]
    fn test_create_permission_keyboard() {
        let keyboard = create_permission_keyboard("abc123", "Bash");
        assert_eq!(keyboard.inline_keyboard.len(), 2);
        assert_eq!(keyboard.inline_keyboard[0].len(), 2); // Allow, Deny
        assert_eq!(keyboard.inline_keyboard[1].len(), 1); // Always Allow
    }
}
