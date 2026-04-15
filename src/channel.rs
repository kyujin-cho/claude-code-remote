//! MCP channel server for two-way communication with Claude Code.
//!
//! This module implements an MCP server that acts as a Claude Code channel,
//! enabling users to send messages from Telegram into a running Claude Code
//! session and receive replies back.
//!
//! Features:
//! - Inbound: Telegram messages → Claude Code session
//! - Outbound: Claude replies via `reply` MCP tool → Telegram
//! - Permission relay: Claude Code permission prompts → Telegram → verdict back

use crate::config::Config;
use crate::messenger::telegram::escape_markdown;
use anyhow::Result;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, CustomNotification, Implementation, ServerInfo};
use rmcp::service::NotificationContext;
use rmcp::{tool, tool_handler, tool_router};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{
    ChatId, InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode, UpdateKind,
};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Parameters for the `reply` tool that Claude calls to send messages back.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ReplyParams {
    /// The message text to send back to the user
    #[schemars(description = "The message text to send back to the user")]
    text: String,
}

/// Parameters for permission request notifications from Claude Code.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PermissionRequestParams {
    request_id: String,
    tool_name: String,
    description: String,
    input_preview: String,
}

/// A permission verdict to send back to Claude Code.
struct PermissionVerdict {
    request_id: String,
    behavior: String, // "allow" or "deny"
}

/// The MCP channel server.
#[derive(Clone)]
struct ChannelServer {
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
    bot: Bot,
    chat_id: ChatId,
    /// Channel for sending permission verdicts back to Claude Code.
    verdict_tx: mpsc::UnboundedSender<PermissionVerdict>,
}

#[tool_router]
impl ChannelServer {
    fn new(
        bot: Bot,
        chat_id: ChatId,
        verdict_tx: mpsc::UnboundedSender<PermissionVerdict>,
    ) -> Self {
        Self {
            tool_router: Self::tool_router(),
            bot,
            chat_id,
            verdict_tx,
        }
    }

    /// Send a message back to the user via Telegram.
    #[tool(description = "Send a reply message to the user via Telegram")]
    async fn reply(
        &self,
        Parameters(ReplyParams { text }): Parameters<ReplyParams>,
    ) -> Result<CallToolResult, McpError> {
        self.bot
            .send_message(self.chat_id, &text)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Failed to send Telegram message: {e}"), None)
            })?;

        Ok(CallToolResult::success(vec![Content::text("sent")]))
    }
}

#[tool_handler]
impl ServerHandler for ChannelServer {
    fn get_info(&self) -> ServerInfo {
        let mut experimental = BTreeMap::new();
        experimental.insert("claude/channel".to_string(), serde_json::Map::new());
        experimental.insert(
            "claude/channel/permission".to_string(),
            serde_json::Map::new(),
        );

        let capabilities = rmcp::model::ServerCapabilities::builder()
            .enable_tools()
            .enable_experimental_with(experimental)
            .build();

        ServerInfo::new(capabilities)
            .with_server_info(Implementation::new(
                "claude-code-telegram",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Messages arrive as <channel source=\"claude-code-telegram\" chat_id=\"...\">. \
             Reply using the reply tool. \
             Permission prompts are forwarded to the user via Telegram with inline buttons; \
             verdicts arrive automatically."
                    .to_string(),
            )
    }

    async fn on_custom_notification(
        &self,
        notification: CustomNotification,
        _context: NotificationContext<RoleServer>,
    ) {
        if notification.method == "notifications/claude/channel/permission_request" {
            if let Some(params) = notification.params {
                if let Ok(req) = serde_json::from_value::<PermissionRequestParams>(params) {
                    self.handle_permission_request(req).await;
                }
            }
        }
    }
}

impl ChannelServer {
    /// Handle an incoming permission request from Claude Code.
    ///
    /// Sends a Telegram message with Allow/Deny buttons and spawns a task
    /// to poll for the callback. The verdict is sent back via the verdict channel.
    async fn handle_permission_request(&self, req: PermissionRequestParams) {
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("✅ Allow", format!("perm:{}:allow", req.request_id)),
            InlineKeyboardButton::callback("❌ Deny", format!("perm:{}:deny", req.request_id)),
        ]]);

        let text = format!(
            "🔐 *Permission Request* `\\[{}\\]`\n\n\
             *Tool:* `{}`\n\
             *Description:* {}\n\n\
             Reply \"yes {}\" or \"no {}\" or tap a button\\.",
            escape_markdown(&req.request_id),
            escape_markdown(&req.tool_name),
            escape_markdown(&req.description),
            escape_markdown(&req.request_id),
            escape_markdown(&req.request_id),
        );

        let sent = match self
            .bot
            .send_message(self.chat_id, &text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await
        {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!("Failed to send permission request to Telegram: {e}");
                return;
            }
        };

        // Spawn a task to poll for the callback button press
        let bot = self.bot.clone();
        let chat_id = self.chat_id;
        let message_id = sent.id;
        let request_id = req.request_id.clone();
        let verdict_tx = self.verdict_tx.clone();

        tokio::spawn(async move {
            let result = poll_permission_callback(
                &bot,
                &request_id,
                message_id,
                chat_id,
                Duration::from_secs(300),
            )
            .await;

            let (behavior, status_text) = match result {
                Some(true) => ("allow", "✅ Approved"),
                Some(false) => ("deny", "❌ Denied"),
                None => ("deny", "⏱️ Timeout \\- Denied"),
            };

            let _ = verdict_tx.send(PermissionVerdict {
                request_id: request_id.clone(),
                behavior: behavior.to_string(),
            });

            // Update the Telegram message with the status
            let new_text = format!("{}\n\n*Status:* {}", text, status_text);
            let _ = bot
                .edit_message_text(chat_id, message_id, new_text)
                .parse_mode(ParseMode::MarkdownV2)
                .await;
        });
    }
}

/// Poll Telegram for a callback query matching a permission request.
///
/// Returns `Some(true)` for allow, `Some(false)` for deny, `None` for timeout.
async fn poll_permission_callback(
    bot: &Bot,
    request_id: &str,
    message_id: MessageId,
    chat_id: ChatId,
    timeout_duration: Duration,
) -> Option<bool> {
    let mut poll_interval = interval(Duration::from_millis(500));
    let mut offset: Option<i32> = None;

    let result = tokio::time::timeout(timeout_duration, async {
        loop {
            poll_interval.tick().await;

            let mut get_updates = bot.get_updates();
            if let Some(off) = offset {
                get_updates = get_updates.offset(off);
            }
            get_updates = get_updates.timeout(5);
            get_updates =
                get_updates.allowed_updates(vec![teloxide::types::AllowedUpdate::CallbackQuery]);

            let updates = match get_updates.await {
                Ok(updates) => updates,
                Err(_) => continue,
            };

            for update in updates {
                offset = Some((update.id.0 + 1) as i32);

                if let UpdateKind::CallbackQuery(query) = update.kind {
                    if let Some(msg) = &query.message {
                        if msg.chat().id != chat_id || msg.id() != message_id {
                            continue;
                        }
                    } else {
                        continue;
                    }

                    if let Some(data) = &query.data {
                        let prefix = format!("perm:{}:", request_id);
                        if let Some(action) = data.strip_prefix(&prefix) {
                            let _ = bot.answer_callback_query(&query.id).await;
                            return matches!(action, "allow");
                        }
                    }
                }
            }
        }
    })
    .await;

    result.ok()
}

/// Poll Telegram for incoming text messages and forward them as channel notifications.
async fn poll_telegram_messages(bot: Bot, chat_id: ChatId, msg_tx: mpsc::UnboundedSender<String>) {
    let mut poll_interval = interval(Duration::from_millis(500));
    let mut offset: Option<i32> = None;

    loop {
        poll_interval.tick().await;

        let mut get_updates = bot.get_updates();
        if let Some(off) = offset {
            get_updates = get_updates.offset(off);
        }
        get_updates = get_updates.timeout(10);
        get_updates = get_updates.allowed_updates(vec![teloxide::types::AllowedUpdate::Message]);

        let updates = match get_updates.await {
            Ok(updates) => updates,
            Err(e) => {
                tracing::warn!("Failed to get Telegram updates: {e}");
                continue;
            }
        };

        for update in updates {
            offset = Some((update.id.0 + 1) as i32);

            if let UpdateKind::Message(msg) = update.kind {
                // Only process messages from our configured chat
                if msg.chat.id != chat_id {
                    continue;
                }

                if let Some(text) = msg.text() {
                    let text = text.to_string();
                    if msg_tx.send(text).is_err() {
                        tracing::error!("Message channel closed");
                        return;
                    }
                }
            }
        }
    }
}

/// Regex for matching permission verdict replies like "yes abcde" or "no abcde".
fn parse_verdict_reply(text: &str) -> Option<PermissionVerdict> {
    let text = text.trim().to_lowercase();
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }

    let behavior = match parts[0] {
        "y" | "yes" => "allow",
        "n" | "no" => "deny",
        _ => return None,
    };

    let request_id = parts[1];
    // Validate it looks like a request ID (lowercase letters, no 'l')
    if request_id.len() != 5
        || !request_id
            .chars()
            .all(|c| c.is_ascii_lowercase() && c != 'l')
    {
        return None;
    }

    Some(PermissionVerdict {
        request_id: request_id.to_string(),
        behavior: behavior.to_string(),
    })
}

/// Run the MCP channel server.
pub async fn run() -> Result<()> {
    let config = Config::load(None)?;
    let telegram = config
        .telegram
        .ok_or_else(|| anyhow::anyhow!("Telegram configuration required for channel mode"))?;

    let bot = Bot::new(&telegram.bot_token);
    let chat_id = telegram.chat_id;

    // Channel for permission verdicts from Telegram callbacks
    let (verdict_tx, mut verdict_rx) = mpsc::unbounded_channel::<PermissionVerdict>();
    // Channel for incoming Telegram text messages
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<String>();

    let server = ChannelServer::new(bot.clone(), chat_id, verdict_tx);

    // Start the MCP server over stdio
    let service = server.serve(rmcp::transport::stdio()).await?;

    // Get a handle to the peer for sending notifications
    let peer = Arc::new(service.peer().clone());

    // Spawn Telegram message polling task
    let poll_bot = bot.clone();
    tokio::spawn(async move {
        poll_telegram_messages(poll_bot, chat_id, msg_tx).await;
    });

    // Spawn task to forward incoming Telegram messages as channel notifications
    let msg_peer = peer.clone();
    tokio::spawn(async move {
        while let Some(text) = msg_rx.recv().await {
            // Check if this is a verdict reply (e.g., "yes abcde")
            if let Some(verdict) = parse_verdict_reply(&text) {
                let notif = CustomNotification::new(
                    "notifications/claude/channel/permission",
                    Some(serde_json::json!({
                        "request_id": verdict.request_id,
                        "behavior": verdict.behavior,
                    })),
                );
                if let Err(e) = msg_peer.send_notification(notif.into()).await {
                    tracing::error!("Failed to send verdict notification: {e}");
                }
                continue;
            }

            // Forward as a channel message
            let notif = CustomNotification::new(
                "notifications/claude/channel",
                Some(serde_json::json!({
                    "content": text,
                    "meta": {
                        "chat_id": chat_id.to_string(),
                    }
                })),
            );
            if let Err(e) = msg_peer.send_notification(notif.into()).await {
                tracing::error!("Failed to send channel notification: {e}");
            }
        }
    });

    // Spawn task to forward permission verdicts from button callbacks
    let verdict_peer = peer.clone();
    tokio::spawn(async move {
        while let Some(verdict) = verdict_rx.recv().await {
            let notif = CustomNotification::new(
                "notifications/claude/channel/permission",
                Some(serde_json::json!({
                    "request_id": verdict.request_id,
                    "behavior": verdict.behavior,
                })),
            );
            if let Err(e) = verdict_peer.send_notification(notif.into()).await {
                tracing::error!("Failed to send verdict notification: {e}");
            }
        }
    });

    // Wait for the MCP service to finish (Claude Code disconnects)
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_verdict_reply_yes() {
        let v = parse_verdict_reply("yes abcde").unwrap();
        assert_eq!(v.request_id, "abcde");
        assert_eq!(v.behavior, "allow");
    }

    #[test]
    fn test_parse_verdict_reply_no() {
        let v = parse_verdict_reply("no abcde").unwrap();
        assert_eq!(v.request_id, "abcde");
        assert_eq!(v.behavior, "deny");
    }

    #[test]
    fn test_parse_verdict_reply_short_forms() {
        let v = parse_verdict_reply("y abcde").unwrap();
        assert_eq!(v.behavior, "allow");
        let v = parse_verdict_reply("n abcde").unwrap();
        assert_eq!(v.behavior, "deny");
    }

    #[test]
    fn test_parse_verdict_reply_case_insensitive() {
        let v = parse_verdict_reply("YES ABCDE").unwrap();
        assert_eq!(v.request_id, "abcde");
        assert_eq!(v.behavior, "allow");
    }

    #[test]
    fn test_parse_verdict_reply_rejects_l() {
        // 'l' is excluded from the ID alphabet
        assert!(parse_verdict_reply("yes abcle").is_none());
    }

    #[test]
    fn test_parse_verdict_reply_rejects_wrong_length() {
        assert!(parse_verdict_reply("yes abc").is_none());
        assert!(parse_verdict_reply("yes abcdef").is_none());
    }

    #[test]
    fn test_parse_verdict_reply_rejects_non_verdict() {
        assert!(parse_verdict_reply("hello world").is_none());
        assert!(parse_verdict_reply("approve abcde").is_none());
        assert!(parse_verdict_reply("yes").is_none());
    }
}
