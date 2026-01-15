//! Error types for the application.

use std::path::PathBuf;
use thiserror::Error;

/// Errors related to configuration loading.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
}

/// Errors related to the always-allow manager.
#[derive(Error, Debug)]
pub enum AlwaysAllowError {
    #[error("Failed to read storage: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Invalid JSON in storage: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

/// Errors related to hook handling.
#[derive(Error, Debug)]
pub enum HookError {
    #[error("Failed to read stdin: {0}")]
    StdinError(#[from] std::io::Error),

    #[error("Invalid hook input: {0}")]
    InvalidInput(#[from] serde_json::Error),

    #[error("Telegram error: {0}")]
    TelegramError(#[from] teloxide::RequestError),

    #[error("Signal error: {0}")]
    #[allow(dead_code)]
    Signal(String),

    #[error("Discord error: {0}")]
    #[allow(dead_code)]
    Discord(String),

    #[error("Timeout waiting for decision")]
    #[allow(dead_code)]
    Timeout,

    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
}

/// Errors related to the stop handler.
#[derive(Error, Debug)]
pub enum StopError {
    #[error("Failed to read stdin: {0}")]
    StdinError(#[from] std::io::Error),

    #[error("Invalid hook input: {0}")]
    InvalidInput(#[from] serde_json::Error),

    #[error("Telegram error: {0}")]
    TelegramError(#[from] teloxide::RequestError),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
}
