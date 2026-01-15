//! Signal messenger implementation.
//!
//! This module provides Signal integration using the presage library.
//! Requires the `signal` feature to be enabled.
//!
//! **Note:** Signal integration does not implement the Messenger trait because
//! presage uses non-Send futures internally. Signal must be used directly.
//!
//! Signal does not support inline keyboards, so users must reply with text commands:
//! - `ALLOW {request_id}` - Allow the permission request
//! - `DENY {request_id}` - Deny the permission request
//! - `ALWAYS {request_id}` - Always allow this tool

use super::{Decision, PermissionMessage};
use crate::error::HookError;
use futures_util::StreamExt;
use presage::libsignal_service::content::ContentBody;
use presage::libsignal_service::prelude::Content;
use presage::libsignal_service::protocol::ServiceId;
use presage::manager::Registered;
use presage::model::messages::Received;
use presage::proto::DataMessage;
use presage::Manager;
use presage_store_sqlite::SqliteStore;
use std::path::Path;
use std::time::Duration;

/// Signal messenger for permission requests.
///
/// Uses presage for Signal protocol implementation.
/// Requires text-based replies since Signal doesn't support inline keyboards.
///
/// **Note:** This does not implement `Messenger` trait because presage uses
/// non-Send futures. Use the methods directly instead.
#[allow(dead_code)]
pub struct SignalMessenger {
    /// Presage manager for Signal operations
    manager: Manager<SqliteStore, Registered>,
    /// Recipient's Signal UUID
    recipient_uuid: uuid::Uuid,
}

#[allow(dead_code)]
impl SignalMessenger {
    /// Create a new Signal messenger from an existing registered manager.
    ///
    /// # Arguments
    /// * `manager` - A registered presage Manager
    /// * `recipient_uuid` - UUID of the recipient to send messages to
    pub fn new(
        manager: Manager<SqliteStore, Registered>,
        recipient_uuid: uuid::Uuid,
    ) -> Result<Self, HookError> {
        Ok(Self {
            manager,
            recipient_uuid,
        })
    }

    /// Load an existing registered manager from storage.
    ///
    /// # Arguments
    /// * `data_path` - Path to the Signal data directory
    /// * `recipient_uuid` - UUID of the recipient to send messages to
    pub async fn from_storage(
        data_path: &Path,
        recipient_uuid: uuid::Uuid,
    ) -> Result<Self, HookError> {
        let db_path = data_path.join("signal.db");
        let db_url = format!("sqlite://{}", db_path.display());

        let store = SqliteStore::open(&db_url, presage_store_sqlite::OnNewIdentity::Trust)
            .await
            .map_err(|e| HookError::Signal(format!("Failed to open Signal store: {}", e)))?;

        let manager = Manager::load_registered(store)
            .await
            .map_err(|e| HookError::Signal(format!("Failed to load Signal manager: {}", e)))?;

        Self::new(manager, recipient_uuid)
    }

    /// Send a text message to the configured recipient.
    async fn send_message(&mut self, text: &str) -> Result<(), HookError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| HookError::Signal(format!("Failed to get timestamp: {}", e)))?
            .as_millis() as u64;

        let data_message = DataMessage {
            body: Some(text.to_string()),
            timestamp: Some(timestamp),
            ..Default::default()
        };

        let content = ContentBody::DataMessage(data_message);
        let service_id = ServiceId::Aci(self.recipient_uuid.into());

        self.manager
            .send_message(service_id, content, timestamp)
            .await
            .map_err(|e| HookError::Signal(format!("Failed to send message: {}", e)))?;

        Ok(())
    }

    /// Poll for incoming messages and look for a matching reply.
    async fn poll_for_reply(
        &mut self,
        request_id: &str,
        poll_timeout: Duration,
    ) -> Result<Decision, HookError> {
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() >= poll_timeout {
                return Ok(Decision::Deny); // Timeout - deny by default
            }

            // Check for new messages
            match self.manager.receive_messages().await {
                Ok(stream) => {
                    // Collect messages with a timeout
                    let collect_future = async {
                        let mut collected = Vec::new();
                        futures_util::pin_mut!(stream);
                        while let Some(item) = stream.next().await {
                            collected.push(item);
                            // Check for QueueEmpty to know we've got all pending messages
                            if matches!(collected.last(), Some(Received::QueueEmpty)) {
                                break;
                            }
                        }
                        collected
                    };

                    let items = tokio::time::timeout(Duration::from_secs(5), collect_future)
                        .await
                        .unwrap_or_default();

                    for item in items {
                        if let Received::Content(content) = item {
                            if let Some(decision) = process_content(&content, request_id) {
                                return Ok(decision);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Error receiving messages: {}", e);
                    // Continue polling despite errors
                }
            }

            // Small delay before next poll
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Send a permission request and wait for user decision.
    pub async fn send_permission_request(
        &mut self,
        message: &PermissionMessage,
        request_timeout: Duration,
    ) -> Result<Decision, HookError> {
        // Format the permission request message
        let text = format_permission_message(message);

        // Send the message
        self.send_message(&text).await?;

        // Poll for reply with timeout
        let decision = tokio::time::timeout(
            request_timeout,
            self.poll_for_reply(&message.request_id, request_timeout),
        )
        .await
        .unwrap_or(Ok(Decision::Deny))?;

        // Send status update
        let status = match decision {
            Decision::Allow => "✅ Approved",
            Decision::Deny => "❌ Denied",
            Decision::AlwaysAllow => "🔓 Always Allowed",
        };

        let _ = self
            .send_message(&format!("Request [{}]: {}", message.request_id, status))
            .await;

        Ok(decision)
    }

    /// Send a notification message.
    pub async fn send_notification(&mut self, text: &str) -> Result<(), HookError> {
        self.send_message(text).await
    }

    /// Send an auto-approved notification.
    pub async fn send_auto_approved(
        &mut self,
        message: &PermissionMessage,
    ) -> Result<(), HookError> {
        let text = format_auto_approved_message(message);
        self.send_message(&text).await
    }

    /// Get the platform name.
    #[allow(dead_code)]
    pub fn platform_name(&self) -> &'static str {
        "Signal"
    }
}

/// Process incoming content and check for a matching decision reply.
#[allow(dead_code)]
fn process_content(content: &Content, request_id: &str) -> Option<Decision> {
    // Extract the body from the content
    if let ContentBody::DataMessage(data_message) = &content.body {
        if let Some(body) = &data_message.body {
            if let Some((decision, reply_id)) = parse_decision_reply(body) {
                // Check if this reply matches our request
                if reply_id.eq_ignore_ascii_case(request_id) {
                    return Some(decision);
                }
            }
        }
    }
    None
}

/// Format a permission request as a Signal message.
#[allow(dead_code)]
fn format_permission_message(message: &PermissionMessage) -> String {
    let mut lines = vec![
        format!("🔐 Permission Request [{}]", message.request_id),
        format!("🖥️ Host: {}", message.hostname),
        String::new(),
        format!("Tool: {}", message.tool_name),
    ];

    match message.tool_name.as_str() {
        "Bash" => {
            if let Some(command) = message.tool_input.get("command").and_then(|v| v.as_str()) {
                let truncated: String = command.chars().take(500).collect();
                lines.push(format!("Command:\n{}", truncated));
            }
        }
        "Edit" | "Write" => {
            if let Some(file_path) = message.tool_input.get("file_path").and_then(|v| v.as_str()) {
                lines.push(format!("File: {}", file_path));
            }

            if message.tool_name == "Edit" {
                if let Some(old_string) = message
                    .tool_input
                    .get("old_string")
                    .and_then(|v| v.as_str())
                {
                    let truncated: String = old_string.chars().take(200).collect();
                    lines.push(format!("Old:\n{}", truncated));
                }
                if let Some(new_string) = message
                    .tool_input
                    .get("new_string")
                    .and_then(|v| v.as_str())
                {
                    let truncated: String = new_string.chars().take(200).collect();
                    lines.push(format!("New:\n{}", truncated));
                }
            }
        }
        _ => {
            let input_str = serde_json::to_string_pretty(&message.tool_input).unwrap_or_default();
            let truncated: String = input_str.chars().take(500).collect();
            lines.push(format!("Input:\n{}", truncated));
        }
    }

    lines.push(String::new());
    lines.push(format!(
        "Reply with:\n• ALLOW {}\n• DENY {}\n• ALWAYS {}",
        message.request_id, message.request_id, message.request_id
    ));

    lines.join("\n")
}

/// Format an auto-approved notification.
#[allow(dead_code)]
fn format_auto_approved_message(message: &PermissionMessage) -> String {
    let mut lines = vec![
        format!("⚙️ Auto-Approved [{}]", message.request_id),
        format!("🖥️ Host: {}", message.hostname),
        String::new(),
        format!("Tool: {} (in always-allow list)", message.tool_name),
    ];

    match message.tool_name.as_str() {
        "Bash" => {
            if let Some(command) = message.tool_input.get("command").and_then(|v| v.as_str()) {
                let truncated: String = command.chars().take(500).collect();
                lines.push(format!("Command:\n{}", truncated));
            }
        }
        "Edit" | "Write" => {
            if let Some(file_path) = message.tool_input.get("file_path").and_then(|v| v.as_str()) {
                lines.push(format!("File: {}", file_path));
            }
        }
        _ => {
            let input_str = serde_json::to_string_pretty(&message.tool_input).unwrap_or_default();
            let truncated: String = input_str.chars().take(500).collect();
            lines.push(format!("Input:\n{}", truncated));
        }
    }

    lines.join("\n")
}

/// Parse a text reply to extract the decision and request ID.
///
/// Expected formats:
/// - `ALLOW abc123`
/// - `DENY abc123`
/// - `ALWAYS abc123`
#[allow(dead_code)]
pub fn parse_decision_reply(text: &str) -> Option<(Decision, String)> {
    let text = text.trim();
    let parts: Vec<&str> = text.split_whitespace().collect();

    if parts.len() < 2 {
        return None;
    }

    let decision = match parts[0].to_uppercase().as_str() {
        "ALLOW" => Decision::Allow,
        "DENY" => Decision::Deny,
        "ALWAYS" => Decision::AlwaysAllow,
        _ => return None,
    };

    let request_id = parts[1].to_string();

    Some((decision, request_id))
}

// ============================================================================
// Device Linking
// ============================================================================

/// Link this device as a secondary device to an existing Signal account.
///
/// This will display a QR code that needs to be scanned from the primary device.
pub async fn link_device(
    data_path: &Path,
    device_name: &str,
) -> Result<Manager<SqliteStore, Registered>, HookError> {
    use futures_channel::oneshot;

    let db_path = data_path.join("signal.db");
    let db_url = format!("sqlite://{}", db_path.display());

    let store = SqliteStore::open(&db_url, presage_store_sqlite::OnNewIdentity::Trust)
        .await
        .map_err(|e| HookError::Signal(format!("Failed to open Signal store: {}", e)))?;

    // Use futures_channel oneshot (required by presage)
    let (provisioning_link_tx, provisioning_link_rx) = oneshot::channel();

    println!("\n📱 Linking device as '{}'...", device_name);
    println!("📂 Data path: {}\n", data_path.display());

    // Run linking - this blocks until QR code is scanned
    // We run it in a single task because presage futures are not Send
    let manager = tokio::task::LocalSet::new()
        .run_until(async move {
            // Spawn local task for the linking process
            let link_handle = tokio::task::spawn_local({
                let device_name = device_name.to_string();
                async move {
                    Manager::link_secondary_device(
                        store,
                        presage::libsignal_service::configuration::SignalServers::Production,
                        device_name,
                        provisioning_link_tx,
                    )
                    .await
                }
            });

            // Wait for provisioning link in another local task
            match provisioning_link_rx.await {
                Ok(link) => {
                    println!("📱 Scan this QR code with your Signal app:\n");

                    // Try to display QR code in terminal
                    if let Ok(code) = qrcode::QrCode::new(link.to_string()) {
                        let string = code
                            .render::<char>()
                            .quiet_zone(true)
                            .module_dimensions(2, 1)
                            .build();
                        println!("{}", string);
                    } else {
                        println!("Link URL: {}", link);
                    }

                    println!("\nWaiting for confirmation...");
                }
                Err(_) => {
                    return Err(HookError::Signal(
                        "Failed to receive provisioning link".to_string(),
                    ));
                }
            }

            // Wait for linking to complete
            link_handle
                .await
                .map_err(|e| HookError::Signal(format!("Task join error: {}", e)))?
                .map_err(|e| HookError::Signal(format!("Failed to link device: {}", e)))
        })
        .await?;

    println!("✅ Device linked successfully!");

    Ok(manager)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_decision_reply_allow() {
        let result = parse_decision_reply("ALLOW abc123").unwrap();
        assert_eq!(result.0, Decision::Allow);
        assert_eq!(result.1, "abc123");
    }

    #[test]
    fn test_parse_decision_reply_deny() {
        let result = parse_decision_reply("deny ABC123").unwrap();
        assert_eq!(result.0, Decision::Deny);
        assert_eq!(result.1, "ABC123");
    }

    #[test]
    fn test_parse_decision_reply_always() {
        let result = parse_decision_reply("Always abc123").unwrap();
        assert_eq!(result.0, Decision::AlwaysAllow);
        assert_eq!(result.1, "abc123");
    }

    #[test]
    fn test_parse_decision_reply_invalid() {
        assert!(parse_decision_reply("invalid").is_none());
        assert!(parse_decision_reply("APPROVE abc123").is_none());
        assert!(parse_decision_reply("").is_none());
    }

    #[test]
    fn test_parse_decision_reply_preserves_case() {
        let result = parse_decision_reply("allow AbC123").unwrap();
        assert_eq!(result.0, Decision::Allow);
        assert_eq!(result.1, "AbC123"); // Request ID case preserved
    }
}
