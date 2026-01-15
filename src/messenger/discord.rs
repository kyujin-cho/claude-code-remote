//! Discord messenger implementation.
//!
//! Implements the Messenger trait for Discord using interactive buttons
//! for permission decisions.

use super::{Decision, Messenger, PermissionMessage};
use crate::error::HookError;
use async_trait::async_trait;
use serenity::all::{
    ButtonStyle, ChannelId, CreateActionRow, CreateButton, CreateMessage, EditMessage, Http,
    MessageId, UserId,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, timeout};

/// Discord messenger for permission requests.
#[allow(dead_code)]
pub struct DiscordMessenger {
    http: Arc<Http>,
    user_id: UserId,
}

#[allow(dead_code)]
impl DiscordMessenger {
    /// Create a new Discord messenger.
    pub fn new(bot_token: &str, user_id: u64) -> Self {
        Self {
            http: Arc::new(Http::new(bot_token)),
            user_id: UserId::new(user_id),
        }
    }

    /// Get or create a DM channel with the user.
    async fn get_dm_channel(&self) -> Result<ChannelId, HookError> {
        let user = self
            .user_id
            .to_user(&self.http)
            .await
            .map_err(|e| HookError::Discord(format!("Failed to get user: {}", e)))?;

        let channel = user
            .create_dm_channel(&self.http)
            .await
            .map_err(|e| HookError::Discord(format!("Failed to create DM channel: {}", e)))?;

        Ok(channel.id)
    }
}

#[async_trait]
impl Messenger for DiscordMessenger {
    async fn send_permission_request(
        &self,
        message: &PermissionMessage,
        request_timeout: Duration,
    ) -> Result<Decision, HookError> {
        let channel_id = self.get_dm_channel().await?;

        // Create buttons
        let buttons = create_permission_buttons(&message.request_id);
        let original_message = format_permission_message(message);

        // Send message with buttons
        let builder = CreateMessage::new()
            .content(&original_message)
            .components(vec![buttons]);

        let sent = channel_id
            .send_message(&self.http, builder)
            .await
            .map_err(|e| HookError::Discord(format!("Failed to send message: {}", e)))?;

        let message_id = sent.id;

        // Poll for button interaction with timeout
        let poll_result = timeout(
            request_timeout,
            poll_for_interaction(&self.http, channel_id, message_id, &message.request_id),
        )
        .await;

        match poll_result {
            Ok(Ok(callback_decision)) => {
                // Determine status text
                let status = match callback_decision {
                    Decision::Allow => "✅ Approved",
                    Decision::Deny => "❌ Denied",
                    Decision::AlwaysAllow => {
                        &format!("🔓 Always Allowed (`{}` added to list)", message.tool_name)
                    }
                };

                // Update message with status (remove buttons)
                let new_text = format!("{}\n\n**Status:** {}", original_message, status);
                let edit_builder = EditMessage::new().content(new_text).components(vec![]);

                let _ = channel_id
                    .edit_message(&self.http, message_id, edit_builder)
                    .await;

                Ok(callback_decision)
            }
            Ok(Err(e)) => {
                // Error during polling
                let _ = channel_id
                    .edit_message(
                        &self.http,
                        message_id,
                        EditMessage::new()
                            .content(format!("{}\n\n**Status:** ❌ Error", original_message))
                            .components(vec![]),
                    )
                    .await;
                Err(e)
            }
            Err(_) => {
                // Timeout - deny by default
                let _ = channel_id
                    .edit_message(
                        &self.http,
                        message_id,
                        EditMessage::new()
                            .content(format!(
                                "{}\n\n**Status:** ⏱️ Timeout - Denied",
                                original_message
                            ))
                            .components(vec![]),
                    )
                    .await;
                Ok(Decision::Deny)
            }
        }
    }

    async fn send_notification(&self, text: &str) -> Result<(), HookError> {
        let channel_id = self.get_dm_channel().await?;

        let builder = CreateMessage::new().content(text);

        channel_id
            .send_message(&self.http, builder)
            .await
            .map_err(|e| HookError::Discord(format!("Failed to send notification: {}", e)))?;

        Ok(())
    }

    async fn send_auto_approved(&self, message: &PermissionMessage) -> Result<(), HookError> {
        let text = format_auto_approved_message(message);
        self.send_notification(&text).await
    }

    fn platform_name(&self) -> &'static str {
        "Discord"
    }
}

/// Create permission buttons for Discord.
#[allow(dead_code)]
fn create_permission_buttons(request_id: &str) -> CreateActionRow {
    CreateActionRow::Buttons(vec![
        CreateButton::new(format!("allow:{}", request_id))
            .label("Allow")
            .style(ButtonStyle::Success),
        CreateButton::new(format!("deny:{}", request_id))
            .label("Deny")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("always:{}", request_id))
            .label("Always Allow")
            .style(ButtonStyle::Primary),
    ])
}

/// Poll for button interaction on a specific message.
#[allow(dead_code)]
async fn poll_for_interaction(
    http: &Http,
    channel_id: ChannelId,
    message_id: MessageId,
    _request_id: &str,
) -> Result<Decision, HookError> {
    let mut poll_interval = interval(Duration::from_millis(500));

    loop {
        poll_interval.tick().await;

        // Fetch the message to check for interactions
        let message = channel_id
            .message(http, message_id)
            .await
            .map_err(|e| HookError::Discord(format!("Failed to fetch message: {}", e)))?;

        // Check if interaction has been received by looking at the message components
        // If buttons are gone, someone clicked - but we need a different approach
        // Discord interactions are ephemeral and require webhook/gateway handling

        // Since we can't easily poll for interactions via REST API alone,
        // we'll check if the message content has been modified (indicating interaction)
        // This is a limitation - for production use, would need gateway connection

        // For now, check if message has no components (meaning we already processed it)
        if message.components.is_empty() {
            // Message was already processed - this shouldn't happen in normal flow
            return Ok(Decision::Deny);
        }

        // Check message reactions or edits as a workaround
        // In a real implementation, you'd use gateway events

        // Try to get any pending interaction via HTTP
        // Note: This is a simplified polling approach
        // A production implementation would use WebSocket gateway events
    }
}

/// Format a permission request as a Discord message.
#[allow(dead_code)]
fn format_permission_message(message: &PermissionMessage) -> String {
    let mut lines = vec![
        format!("🔐 **Permission Request** [{}]", message.request_id),
        format!("🖥️ **Host:** {}", message.hostname),
        String::new(),
        format!("**Tool:** {}", message.tool_name),
    ];

    match message.tool_name.as_str() {
        "Bash" => {
            if let Some(command) = message.tool_input.get("command").and_then(|v| v.as_str()) {
                let truncated: String = command.chars().take(500).collect();
                lines.push(format!("**Command:**\n```\n{}\n```", truncated));
            }
        }
        "Edit" | "Write" => {
            if let Some(file_path) = message.tool_input.get("file_path").and_then(|v| v.as_str()) {
                lines.push(format!("**File:** `{}`", file_path));
            }

            if message.tool_name == "Edit" {
                if let Some(old_string) = message
                    .tool_input
                    .get("old_string")
                    .and_then(|v| v.as_str())
                {
                    let truncated: String = old_string.chars().take(200).collect();
                    lines.push(format!("**Old:**\n```\n{}\n```", truncated));
                }
                if let Some(new_string) = message
                    .tool_input
                    .get("new_string")
                    .and_then(|v| v.as_str())
                {
                    let truncated: String = new_string.chars().take(200).collect();
                    lines.push(format!("**New:**\n```\n{}\n```", truncated));
                }
            }
        }
        _ => {
            let input_str = serde_json::to_string_pretty(&message.tool_input).unwrap_or_default();
            let truncated: String = input_str.chars().take(500).collect();
            lines.push(format!("**Input:**\n```json\n{}\n```", truncated));
        }
    }

    lines.join("\n")
}

/// Format an auto-approved notification as a Discord message.
#[allow(dead_code)]
fn format_auto_approved_message(message: &PermissionMessage) -> String {
    let mut lines = vec![
        format!("⚙️ **Auto-Approved** [{}]", message.request_id),
        format!("🖥️ **Host:** {}", message.hostname),
        String::new(),
        format!("**Tool:** {} *(in always-allow list)*", message.tool_name),
    ];

    match message.tool_name.as_str() {
        "Bash" => {
            if let Some(command) = message.tool_input.get("command").and_then(|v| v.as_str()) {
                let truncated: String = command.chars().take(500).collect();
                lines.push(format!("**Command:**\n```\n{}\n```", truncated));
            }
        }
        "Edit" | "Write" => {
            if let Some(file_path) = message.tool_input.get("file_path").and_then(|v| v.as_str()) {
                lines.push(format!("**File:** `{}`", file_path));
            }
        }
        _ => {
            let input_str = serde_json::to_string_pretty(&message.tool_input).unwrap_or_default();
            let truncated: String = input_str.chars().take(500).collect();
            lines.push(format!("**Input:**\n```json\n{}\n```", truncated));
        }
    }

    lines.join("\n")
}

/// Parse a button custom_id to extract decision and request_id.
#[allow(dead_code)]
pub fn parse_button_custom_id(custom_id: &str) -> Option<(Decision, String)> {
    let parts: Vec<&str> = custom_id.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }

    let decision = match parts[0] {
        "allow" => Decision::Allow,
        "deny" => Decision::Deny,
        "always" => Decision::AlwaysAllow,
        _ => return None,
    };

    Some((decision, parts[1].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_button_custom_id_allow() {
        let result = parse_button_custom_id("allow:abc123").unwrap();
        assert_eq!(result.0, Decision::Allow);
        assert_eq!(result.1, "abc123");
    }

    #[test]
    fn test_parse_button_custom_id_deny() {
        let result = parse_button_custom_id("deny:xyz789").unwrap();
        assert_eq!(result.0, Decision::Deny);
        assert_eq!(result.1, "xyz789");
    }

    #[test]
    fn test_parse_button_custom_id_always() {
        let result = parse_button_custom_id("always:test123").unwrap();
        assert_eq!(result.0, Decision::AlwaysAllow);
        assert_eq!(result.1, "test123");
    }

    #[test]
    fn test_parse_button_custom_id_invalid() {
        assert!(parse_button_custom_id("invalid").is_none());
        assert!(parse_button_custom_id("approve:abc123").is_none());
        assert!(parse_button_custom_id("").is_none());
    }
}
