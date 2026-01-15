//! Stop handler for job completion notifications.
//!
//! Handles Stop hook events by sending notifications via configured messengers
//! when Claude Code finishes a task.

use crate::config::Config;
use crate::error::StopError;
use crate::messenger::telegram::TelegramMessenger;
use crate::messenger::Messenger;
use serde::Deserialize;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::PathBuf;

#[cfg(feature = "discord")]
use crate::messenger::discord::DiscordMessenger;

/// Claude Code stop hook input.
#[derive(Debug, Deserialize)]
pub struct StopInput {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub transcript_path: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub stop_hook_active: bool,
}

/// Stop event with parsed data.
#[derive(Debug)]
pub struct StopEvent {
    #[allow(dead_code)]
    pub session_id: String,
    pub transcript_path: PathBuf,
    pub cwd: PathBuf,
    pub stop_hook_active: bool,
}

impl StopEvent {
    /// Create a stop event from hook input.
    pub fn from_input(input: StopInput) -> Self {
        Self {
            session_id: input.session_id,
            transcript_path: PathBuf::from(input.transcript_path),
            cwd: PathBuf::from(input.cwd),
            stop_hook_active: input.stop_hook_active,
        }
    }

    /// Get the last assistant message from the transcript.
    pub fn get_last_assistant_message(&self) -> Option<String> {
        if self.transcript_path.as_os_str().is_empty() {
            return None;
        }

        if !self.transcript_path.exists() {
            return None;
        }

        let file = File::open(&self.transcript_path).ok()?;
        let reader = BufReader::new(file);

        let mut last_message: Option<String> = None;

        for line in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<TranscriptEntry>(&line) {
                if entry.entry_type == "assistant" {
                    if let Some(message) = entry.message {
                        for block in message.content {
                            if let ContentBlock::Text { text } = block {
                                last_message = Some(text);
                            }
                        }
                    }
                }
            }
        }

        last_message
    }

    /// Get the project name from the current working directory.
    pub fn get_project_name(&self) -> String {
        self.cwd
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }
}

/// Transcript entry structure.
#[derive(Debug, Deserialize)]
struct TranscriptEntry {
    #[serde(rename = "type")]
    entry_type: String,
    #[serde(default)]
    message: Option<TranscriptMessage>,
}

#[derive(Debug, Deserialize)]
struct TranscriptMessage {
    #[serde(default)]
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

/// Format job completion message.
fn format_completion_message(config: &Config, event: &StopEvent) -> String {
    let project_name = event.get_project_name();

    let mut lines = vec![
        "✅ **Job Completed**".to_string(),
        format!("🖥️ **Host:** {}", config.hostname),
        format!("📁 **Project:** {}", project_name),
    ];

    // Try to get last assistant message for summary
    if let Some(last_message) = event.get_last_assistant_message() {
        let truncated: String = last_message.chars().take(300).collect();
        let summary = if last_message.len() > 300 {
            format!("{}...", truncated)
        } else {
            truncated
        };
        lines.push(String::new());
        lines.push(format!("**Summary:**\n{}", summary));
    }

    lines.join("\n")
}

/// Send job completion notification via configured messenger.
pub async fn send_notification(config: &Config, event: &StopEvent) -> Result<(), StopError> {
    // Skip if this is a continuation from a stop hook to prevent loops
    if event.stop_hook_active {
        return Ok(());
    }

    let text = format_completion_message(config, event);

    // Try Discord if configured as primary
    #[cfg(feature = "discord")]
    if config.primary_messenger == "discord" {
        if let Some(ref discord_config) = config.discord {
            if discord_config.enabled {
                let messenger =
                    DiscordMessenger::new(&discord_config.bot_token, discord_config.user_id);
                messenger.send_notification(&text).await.map_err(|e| {
                    StopError::TelegramError(teloxide::RequestError::Api(
                        teloxide::ApiError::Unknown(e.to_string()),
                    ))
                })?;
                return Ok(());
            }
        }
    }

    // Try Telegram if configured
    if let Some(ref telegram_config) = config.telegram {
        let messenger = TelegramMessenger::new(&telegram_config.bot_token, telegram_config.chat_id);
        messenger.send_notification(&text).await.map_err(|e| {
            StopError::TelegramError(teloxide::RequestError::Api(teloxide::ApiError::Unknown(
                e.to_string(),
            )))
        })?;
        return Ok(());
    }

    // Try Discord as fallback
    #[cfg(feature = "discord")]
    if let Some(ref discord_config) = config.discord {
        if discord_config.enabled {
            let messenger =
                DiscordMessenger::new(&discord_config.bot_token, discord_config.user_id);
            messenger.send_notification(&text).await.map_err(|e| {
                StopError::TelegramError(teloxide::RequestError::Api(teloxide::ApiError::Unknown(
                    e.to_string(),
                )))
            })?;
            return Ok(());
        }
    }

    // No messenger configured - silently skip
    Ok(())
}

/// Read JSON input from stdin.
fn read_stdin() -> Result<String, io::Error> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Main entry point for the stop handler.
pub async fn run() -> Result<(), StopError> {
    // Read and parse input
    let input_str = read_stdin()?;
    let input: StopInput = serde_json::from_str(&input_str)?;

    // Load config
    let config = Config::load(None)?;

    // Create event and send notification
    let event = StopEvent::from_input(input);
    send_notification(&config, &event).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_stop_event_from_input() {
        let input = StopInput {
            session_id: "abc123".to_string(),
            transcript_path: "/path/to/transcript.jsonl".to_string(),
            cwd: "/home/user/project".to_string(),
            stop_hook_active: false,
        };

        let event = StopEvent::from_input(input);
        assert_eq!(event.session_id, "abc123");
        assert_eq!(event.cwd, PathBuf::from("/home/user/project"));
        assert!(!event.stop_hook_active);
    }

    #[test]
    fn test_get_project_name() {
        let event = StopEvent {
            session_id: String::new(),
            transcript_path: PathBuf::new(),
            cwd: PathBuf::from("/home/user/my-project"),
            stop_hook_active: false,
        };

        assert_eq!(event.get_project_name(), "my-project");
    }

    #[test]
    fn test_get_last_assistant_message_empty_path() {
        let event = StopEvent {
            session_id: String::new(),
            transcript_path: PathBuf::new(),
            cwd: PathBuf::new(),
            stop_hook_active: false,
        };

        assert!(event.get_last_assistant_message().is_none());
    }

    #[test]
    fn test_get_last_assistant_message_nonexistent_file() {
        let event = StopEvent {
            session_id: String::new(),
            transcript_path: PathBuf::from("/nonexistent/path.jsonl"),
            cwd: PathBuf::new(),
            stop_hook_active: false,
        };

        assert!(event.get_last_assistant_message().is_none());
    }

    #[test]
    fn test_get_last_assistant_message_valid_transcript() {
        let dir = tempdir().unwrap();
        let transcript_path = dir.path().join("transcript.jsonl");

        let mut file = File::create(&transcript_path).unwrap();
        writeln!(
            file,
            r#"{{"type": "user", "message": {{"content": [{{"type": "text", "text": "Hello"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type": "assistant", "message": {{"content": [{{"type": "text", "text": "First response"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type": "assistant", "message": {{"content": [{{"type": "text", "text": "Final response"}}]}}}}"#
        )
        .unwrap();

        let event = StopEvent {
            session_id: String::new(),
            transcript_path,
            cwd: PathBuf::new(),
            stop_hook_active: false,
        };

        assert_eq!(
            event.get_last_assistant_message(),
            Some("Final response".to_string())
        );
    }
}
