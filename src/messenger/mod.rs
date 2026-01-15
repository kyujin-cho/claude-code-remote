//! Messenger abstraction layer for multi-platform support.
//!
//! Provides a trait-based abstraction over messaging platforms (Telegram, Signal, etc.)
//! to enable sending permission requests and receiving user decisions.

mod types;

pub mod telegram;

#[cfg(feature = "signal")]
pub mod signal;

#[cfg(feature = "discord")]
pub mod discord;

pub use types::{Decision, PermissionMessage};

use crate::error::HookError;
use async_trait::async_trait;
use std::time::Duration;

/// Abstraction over messaging platforms for permission request handling.
#[async_trait]
pub trait Messenger: Send + Sync {
    /// Send a permission request and wait for user decision.
    ///
    /// # Arguments
    /// * `message` - The permission request details
    /// * `timeout` - Maximum time to wait for a response
    ///
    /// # Returns
    /// The user's decision (Allow, Deny, or AlwaysAllow)
    async fn send_permission_request(
        &self,
        message: &PermissionMessage,
        timeout: Duration,
    ) -> Result<Decision, HookError>;

    /// Send a notification message (no response expected).
    ///
    /// Used for auto-approved notifications and job completion alerts.
    async fn send_notification(&self, text: &str) -> Result<(), HookError>;

    /// Send an auto-approved notification with request details.
    async fn send_auto_approved(&self, message: &PermissionMessage) -> Result<(), HookError>;

    /// Get the platform name for logging purposes.
    #[allow(dead_code)]
    fn platform_name(&self) -> &'static str;
}
