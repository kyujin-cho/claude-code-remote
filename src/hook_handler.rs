//! Permission request handler for Claude Code hooks.
//!
//! Handles PermissionRequest hook events by sending messages via configured
//! messenger (Telegram, Signal, Discord) with interactive decision options.

use crate::always_allow::AlwaysAllowManager;
use crate::config::Config;
use crate::error::HookError;
#[cfg(feature = "discord")]
use crate::messenger::discord::DiscordMessenger;
use crate::messenger::telegram::TelegramMessenger;
use crate::messenger::{Decision, Messenger, PermissionMessage};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Read};
use std::time::Duration;

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

    /// Convert to a PermissionMessage for sending via messenger.
    pub fn to_message(&self, hostname: &str) -> PermissionMessage {
        PermissionMessage::new(
            self.request_id.clone(),
            self.tool_name.clone(),
            hostname.to_string(),
            self.tool_input.clone(),
        )
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

/// Handle a permission request using the provided messenger.
///
/// This is the main entry point for processing permission requests.
/// It checks the always-allow list first, then sends a message via
/// the messenger and waits for user decision.
pub async fn handle_permission_request_with_messenger<M: Messenger>(
    messenger: &M,
    always_allow: &AlwaysAllowManager,
    request: &PermissionRequest,
    hostname: &str,
    request_timeout: Duration,
) -> Result<Decision, HookError> {
    let message = request.to_message(hostname);

    // Check if tool is in always-allow list
    if always_allow.is_allowed(&request.tool_name) {
        messenger.send_auto_approved(&message).await?;
        return Ok(Decision::Allow);
    }

    // Send permission request and wait for decision
    let decision = messenger
        .send_permission_request(&message, request_timeout)
        .await?;

    // Handle always allow
    if decision == Decision::AlwaysAllow {
        let _ = always_allow.add_tool(&request.tool_name);
        return Ok(Decision::Allow);
    }

    Ok(decision)
}

/// Handle a permission request using the configured primary messenger.
///
/// Selects between Telegram, Discord, or Signal based on config.primary_messenger.
pub async fn handle_permission_request(
    config: &Config,
    always_allow: &AlwaysAllowManager,
    request: &PermissionRequest,
) -> Result<Decision, HookError> {
    let timeout = Duration::from_secs(config.timeout_seconds);

    // Try Discord if configured as primary
    #[cfg(feature = "discord")]
    if config.primary_messenger == "discord" {
        if let Some(ref discord_config) = config.discord {
            if discord_config.enabled {
                let messenger =
                    DiscordMessenger::new(&discord_config.bot_token, discord_config.user_id);
                return handle_permission_request_with_messenger(
                    &messenger,
                    always_allow,
                    request,
                    &config.hostname,
                    timeout,
                )
                .await;
            }
        }
    }

    // Try Telegram if configured as primary or as fallback
    if let Some(ref telegram_config) = config.telegram {
        let messenger = TelegramMessenger::new(&telegram_config.bot_token, telegram_config.chat_id);
        return handle_permission_request_with_messenger(
            &messenger,
            always_allow,
            request,
            &config.hostname,
            timeout,
        )
        .await;
    }

    // Try Discord as fallback if telegram not available
    #[cfg(feature = "discord")]
    if let Some(ref discord_config) = config.discord {
        if discord_config.enabled {
            let messenger =
                DiscordMessenger::new(&discord_config.bot_token, discord_config.user_id);
            return handle_permission_request_with_messenger(
                &messenger,
                always_allow,
                request,
                &config.hostname,
                timeout,
            )
            .await;
        }
    }

    // No messenger available
    Err(HookError::ConfigError(
        crate::error::ConfigError::MissingField("no messenger configured".to_string()),
    ))
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
    fn test_permission_request_to_message() {
        let request = PermissionRequest {
            tool_name: "Bash".to_string(),
            tool_input: serde_json::json!({"command": "ls -la"}),
            request_id: "abc12345".to_string(),
        };

        let message = request.to_message("test-host");
        assert_eq!(message.tool_name, "Bash");
        assert_eq!(message.hostname, "test-host");
        assert_eq!(message.request_id, "abc12345");
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
