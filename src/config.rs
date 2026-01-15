//! Configuration management for messenger integrations.
//!
//! Supports two configuration formats:
//! 1. Legacy format: `~/.claude/telegram_hook.json` with `telegram_bot_token` and `telegram_chat_id`
//! 2. New format: `~/.claude/hook_config.json` with `messengers` section for Telegram and Signal
//!
//! Falls back to environment variables if no config file exists.

use crate::error::ConfigError;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use teloxide::types::ChatId;

/// Default configuration file path (new format).
pub fn default_config_path() -> PathBuf {
    dirs_config_dir().join("hook_config.json")
}

/// Legacy configuration file path (old format).
pub fn legacy_config_path() -> PathBuf {
    dirs_config_dir().join("telegram_hook.json")
}

/// Default always-allow file path.
pub fn default_always_allow_path() -> PathBuf {
    dirs_config_dir().join("always_allow.json")
}

/// Default Signal data directory path.
#[cfg(feature = "signal")]
pub fn default_signal_data_path() -> PathBuf {
    dirs_config_dir().join("signal_data")
}

/// Get the .claude config directory path.
fn dirs_config_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(".claude"))
        .unwrap_or_else(|| PathBuf::from(".claude"))
}

// ============================================================================
// Legacy Configuration (backward compatibility)
// ============================================================================

/// Legacy JSON configuration file structure.
#[derive(Debug, Deserialize)]
struct LegacyConfigFile {
    telegram_bot_token: String,
    telegram_chat_id: ChatIdValue,
}

/// Chat ID that can be either string or integer in JSON.
#[derive(Debug, Clone, Deserialize)]
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

// ============================================================================
// New Configuration Format
// ============================================================================

/// New JSON configuration file structure with multi-messenger support.
#[derive(Debug, Deserialize)]
struct NewConfigFile {
    messengers: MessengersConfig,
    #[serde(default)]
    preferences: PreferencesConfig,
}

/// Configuration for all supported messengers.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MessengersConfig {
    #[serde(default)]
    telegram: Option<TelegramConfigFile>,
    #[serde(default)]
    signal: Option<SignalConfigFile>,
    #[cfg(feature = "discord")]
    #[serde(default)]
    discord: Option<DiscordConfigFile>,
}

/// Telegram-specific configuration from file.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TelegramConfigFile {
    #[serde(default = "default_enabled")]
    enabled: bool,
    bot_token: String,
    chat_id: ChatIdValue,
}

/// Signal-specific configuration from file.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SignalConfigFile {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub phone_number: String,
    #[serde(default = "default_device_name")]
    pub device_name: String,
    #[serde(default)]
    pub data_path: Option<String>,
}

/// Discord-specific configuration from file.
#[cfg(feature = "discord")]
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DiscordConfigFile {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub bot_token: String,
    pub user_id: DiscordUserIdValue,
}

/// Discord user ID that can be either string or integer in JSON.
#[cfg(feature = "discord")]
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DiscordUserIdValue {
    String(String),
    Integer(u64),
}

#[cfg(feature = "discord")]
impl DiscordUserIdValue {
    pub fn to_u64(&self) -> Result<u64, ConfigError> {
        match self {
            DiscordUserIdValue::String(s) => s.parse::<u64>().map_err(|_| {
                ConfigError::MissingField("discord.user_id must be a valid integer".to_string())
            }),
            DiscordUserIdValue::Integer(i) => Ok(*i),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_device_name() -> String {
    "claude-code-hook".to_string()
}

/// User preferences for messenger behavior.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PreferencesConfig {
    #[serde(default = "default_primary_messenger")]
    primary_messenger: String,
    #[serde(default = "default_timeout_seconds")]
    timeout_seconds: u64,
}

impl Default for PreferencesConfig {
    fn default() -> Self {
        Self {
            primary_messenger: default_primary_messenger(),
            timeout_seconds: default_timeout_seconds(),
        }
    }
}

fn default_primary_messenger() -> String {
    "telegram".to_string()
}

fn default_timeout_seconds() -> u64 {
    300
}

// ============================================================================
// Application Configuration
// ============================================================================

/// Telegram configuration.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: ChatId,
}

/// Signal configuration.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SignalConfig {
    pub enabled: bool,
    pub phone_number: String,
    pub device_name: String,
    pub data_path: PathBuf,
}

/// Discord configuration.
#[cfg(feature = "discord")]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub user_id: u64,
}

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// System hostname
    pub hostname: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Primary messenger to use ("telegram", "discord", "signal")
    pub primary_messenger: String,
    /// Optional Telegram configuration
    pub telegram: Option<TelegramConfig>,
    /// Optional Signal configuration (only with signal feature)
    #[cfg(feature = "signal")]
    pub signal: Option<SignalConfig>,
    /// Optional Discord configuration (only with discord feature)
    #[cfg(feature = "discord")]
    pub discord: Option<DiscordConfig>,
}

impl Config {
    /// Load configuration from JSON file, falling back to environment variables.
    ///
    /// Search order:
    /// 1. Provided config_path (if any)
    /// 2. New format: `~/.claude/hook_config.json`
    /// 3. Legacy format: `~/.claude/telegram_hook.json`
    /// 4. Environment variables
    pub fn load(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        // If a specific path is provided, use it
        if let Some(path) = config_path {
            if path.exists() {
                return Self::from_json(&path);
            }
        }

        // Try new config format first
        let new_path = default_config_path();
        if new_path.exists() {
            return Self::from_json(&new_path);
        }

        // Fall back to legacy config
        let legacy_path = legacy_config_path();
        if legacy_path.exists() {
            return Self::from_json(&legacy_path);
        }

        // Fall back to environment variables
        Self::from_env()
    }

    /// Load configuration from a JSON file.
    ///
    /// Automatically detects whether it's the new or legacy format.
    pub fn from_json(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }

        let content = fs::read_to_string(path)?;

        // Try new format first (has "messengers" key)
        if let Ok(new_config) = serde_json::from_str::<NewConfigFile>(&content) {
            return Self::from_new_format(new_config);
        }

        // Fall back to legacy format
        let legacy_config: LegacyConfigFile = serde_json::from_str(&content)?;
        Self::from_legacy_format(legacy_config)
    }

    /// Parse new configuration format.
    fn from_new_format(config: NewConfigFile) -> Result<Self, ConfigError> {
        let hostname = get_hostname();

        // Parse telegram config (optional)
        let telegram = config
            .messengers
            .telegram
            .filter(|t| t.enabled && !t.bot_token.is_empty())
            .map(|t| {
                t.chat_id.to_chat_id().map(|chat_id| TelegramConfig {
                    bot_token: t.bot_token,
                    chat_id,
                })
            })
            .transpose()?;

        #[cfg(feature = "signal")]
        let signal = config
            .messengers
            .signal
            .filter(|s| s.enabled)
            .map(|s| SignalConfig {
                enabled: s.enabled,
                phone_number: s.phone_number,
                device_name: s.device_name,
                data_path: s
                    .data_path
                    .map(PathBuf::from)
                    .unwrap_or_else(default_signal_data_path),
            });

        #[cfg(feature = "discord")]
        let discord = config
            .messengers
            .discord
            .filter(|d| d.enabled)
            .map(|d| {
                d.user_id.to_u64().map(|user_id| DiscordConfig {
                    enabled: d.enabled,
                    bot_token: d.bot_token,
                    user_id,
                })
            })
            .transpose()?;

        // Validate that at least one messenger is configured
        let has_messenger = telegram.is_some();
        #[cfg(feature = "discord")]
        let has_messenger = has_messenger || discord.is_some();
        #[cfg(feature = "signal")]
        let has_messenger = has_messenger || signal.is_some();

        if !has_messenger {
            return Err(ConfigError::MissingField(
                "at least one messenger must be configured".to_string(),
            ));
        }

        Ok(Self {
            hostname,
            timeout_seconds: config.preferences.timeout_seconds,
            primary_messenger: config.preferences.primary_messenger,
            telegram,
            #[cfg(feature = "signal")]
            signal,
            #[cfg(feature = "discord")]
            discord,
        })
    }

    /// Parse legacy configuration format.
    fn from_legacy_format(config: LegacyConfigFile) -> Result<Self, ConfigError> {
        if config.telegram_bot_token.is_empty() {
            return Err(ConfigError::MissingField("telegram_bot_token".to_string()));
        }

        let chat_id = config.telegram_chat_id.to_chat_id()?;
        let hostname = get_hostname();

        Ok(Self {
            hostname,
            timeout_seconds: default_timeout_seconds(),
            primary_messenger: default_primary_messenger(),
            telegram: Some(TelegramConfig {
                bot_token: config.telegram_bot_token,
                chat_id,
            }),
            #[cfg(feature = "signal")]
            signal: None,
            #[cfg(feature = "discord")]
            discord: None,
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

        let hostname = get_hostname();

        Ok(Self {
            hostname,
            timeout_seconds: default_timeout_seconds(),
            primary_messenger: default_primary_messenger(),
            telegram: Some(TelegramConfig {
                bot_token: token,
                chat_id,
            }),
            #[cfg(feature = "signal")]
            signal: None,
            #[cfg(feature = "discord")]
            discord: None,
        })
    }
}

/// Get system hostname.
fn get_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // =========================================================================
    // Legacy Format Tests
    // =========================================================================

    #[test]
    fn test_legacy_config_with_string_chat_id() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"telegram_bot_token":"test_token","telegram_chat_id":"123456"}"#,
        )
        .unwrap();

        let config = Config::from_json(&config_path).unwrap();
        let telegram = config.telegram.expect("telegram should be configured");
        assert_eq!(telegram.bot_token, "test_token");
        assert_eq!(telegram.chat_id, ChatId(123456));
        assert_eq!(config.timeout_seconds, 300); // Default
    }

    #[test]
    fn test_legacy_config_with_int_chat_id() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"telegram_bot_token":"test_token","telegram_chat_id":123456}"#,
        )
        .unwrap();

        let config = Config::from_json(&config_path).unwrap();
        let telegram = config.telegram.expect("telegram should be configured");
        assert_eq!(telegram.bot_token, "test_token");
        assert_eq!(telegram.chat_id, ChatId(123456));
    }

    #[test]
    fn test_legacy_config_missing_token() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(&config_path, r#"{"telegram_chat_id":"123456"}"#).unwrap();

        let result = Config::from_json(&config_path);
        assert!(result.is_err());
    }

    // =========================================================================
    // New Format Tests
    // =========================================================================

    #[test]
    fn test_new_config_telegram_only() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{
                "messengers": {
                    "telegram": {
                        "bot_token": "new_token",
                        "chat_id": "789012"
                    }
                }
            }"#,
        )
        .unwrap();

        let config = Config::from_json(&config_path).unwrap();
        let telegram = config.telegram.expect("telegram should be configured");
        assert_eq!(telegram.bot_token, "new_token");
        assert_eq!(telegram.chat_id, ChatId(789012));
        assert_eq!(config.timeout_seconds, 300); // Default
    }

    #[test]
    fn test_new_config_with_preferences() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{
                "messengers": {
                    "telegram": {
                        "enabled": true,
                        "bot_token": "token123",
                        "chat_id": 111222
                    }
                },
                "preferences": {
                    "primary_messenger": "telegram",
                    "timeout_seconds": 600
                }
            }"#,
        )
        .unwrap();

        let config = Config::from_json(&config_path).unwrap();
        let telegram = config.telegram.expect("telegram should be configured");
        assert_eq!(telegram.bot_token, "token123");
        assert_eq!(telegram.chat_id, ChatId(111222));
        assert_eq!(config.timeout_seconds, 600);
    }

    #[test]
    fn test_new_config_missing_telegram() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{
                "messengers": {}
            }"#,
        )
        .unwrap();

        let result = Config::from_json(&config_path);
        assert!(result.is_err());
    }

    // =========================================================================
    // General Tests
    // =========================================================================

    #[test]
    fn test_config_file_not_found() {
        let result = Config::from_json(Path::new("/nonexistent/path.json"));
        assert!(matches!(result, Err(ConfigError::FileNotFound(_))));
    }

    // Backward compatibility aliases for existing tests
    #[test]
    fn test_config_from_json_with_string_chat_id() {
        test_legacy_config_with_string_chat_id();
    }

    #[test]
    fn test_config_from_json_with_int_chat_id() {
        test_legacy_config_with_int_chat_id();
    }

    #[test]
    fn test_config_from_json_missing_token() {
        test_legacy_config_missing_token();
    }
}
