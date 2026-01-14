//! Configuration management for the Telegram bot.
//!
//! Loads configuration from JSON file (~/.claude/telegram_hook.json) with
//! fallback to environment variables.

use crate::error::ConfigError;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use teloxide::types::ChatId;

/// Default configuration file path.
pub fn default_config_path() -> PathBuf {
    dirs_config_dir().join("telegram_hook.json")
}

/// Default always-allow file path.
pub fn default_always_allow_path() -> PathBuf {
    dirs_config_dir().join("always_allow.json")
}

/// Get the .claude config directory path.
fn dirs_config_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(".claude"))
        .unwrap_or_else(|| PathBuf::from(".claude"))
}

/// JSON configuration file structure.
#[derive(Debug, Deserialize)]
struct ConfigFile {
    telegram_bot_token: String,
    telegram_chat_id: ChatIdValue,
}

/// Chat ID that can be either string or integer in JSON.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ChatIdValue {
    String(String),
    Integer(i64),
}

impl ChatIdValue {
    fn to_chat_id(&self) -> Result<ChatId, ConfigError> {
        match self {
            ChatIdValue::String(s) => s.parse::<i64>().map(ChatId).map_err(|_| {
                ConfigError::MissingField("telegram_chat_id must be a valid integer".to_string())
            }),
            ChatIdValue::Integer(i) => Ok(ChatId(*i)),
        }
    }
}

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub telegram_bot_token: String,
    pub telegram_chat_id: ChatId,
    pub hostname: String,
}

impl Config {
    /// Load configuration from JSON file, falling back to environment variables.
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let path = config_path.unwrap_or_else(default_config_path);

        if path.exists() {
            Self::from_json(&path)
        } else {
            Self::from_env()
        }
    }

    /// Load configuration from a JSON file.
    pub fn from_json(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }

        let content = fs::read_to_string(path)?;
        let config_file: ConfigFile = serde_json::from_str(&content)?;

        // Validate required fields
        if config_file.telegram_bot_token.is_empty() {
            return Err(ConfigError::MissingField("telegram_bot_token".to_string()));
        }

        let chat_id = config_file.telegram_chat_id.to_chat_id()?;
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(Self {
            telegram_bot_token: config_file.telegram_bot_token,
            telegram_chat_id: chat_id,
            hostname,
        })
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        // Try to load .env file (silently ignore if not found)
        let _ = dotenvy::from_path(dirs_config_dir().join(".env"));

        let token = env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| ConfigError::MissingEnvVar("TELEGRAM_BOT_TOKEN".to_string()))?;

        let chat_id_str = env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| ConfigError::MissingEnvVar("TELEGRAM_CHAT_ID".to_string()))?;

        let chat_id = chat_id_str.parse::<i64>().map(ChatId).map_err(|_| {
            ConfigError::MissingField("TELEGRAM_CHAT_ID must be a valid integer".to_string())
        })?;

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(Self {
            telegram_bot_token: token,
            telegram_chat_id: chat_id,
            hostname,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_from_json_with_string_chat_id() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"telegram_bot_token":"test_token","telegram_chat_id":"123456"}"#,
        )
        .unwrap();

        let config = Config::from_json(&config_path).unwrap();
        assert_eq!(config.telegram_bot_token, "test_token");
        assert_eq!(config.telegram_chat_id, ChatId(123456));
    }

    #[test]
    fn test_config_from_json_with_int_chat_id() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"telegram_bot_token":"test_token","telegram_chat_id":123456}"#,
        )
        .unwrap();

        let config = Config::from_json(&config_path).unwrap();
        assert_eq!(config.telegram_bot_token, "test_token");
        assert_eq!(config.telegram_chat_id, ChatId(123456));
    }

    #[test]
    fn test_config_from_json_missing_token() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(&config_path, r#"{"telegram_chat_id":"123456"}"#).unwrap();

        let result = Config::from_json(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_file_not_found() {
        let result = Config::from_json(Path::new("/nonexistent/path.json"));
        assert!(matches!(result, Err(ConfigError::FileNotFound(_))));
    }
}
