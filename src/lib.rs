//! Claude Code Telegram Bot library.
//!
//! This library provides the core functionality for the Claude Code messaging integration.
//! Supports Telegram, Discord (with the `discord` feature), and Signal (with the `signal` feature).

pub mod always_allow;
pub mod bot;
#[cfg(feature = "channel")]
pub mod channel;
pub mod cli;
pub mod config;
pub mod error;
pub mod event_handler;
pub mod hook_handler;
pub mod messenger;
pub mod notification_handler;
pub mod stop_handler;
pub mod telegram;
pub mod util;

// Re-export commonly used types
pub use always_allow::AlwaysAllowManager;
pub use config::Config;
pub use hook_handler::{HookInput, HookOutput, PermissionRequest};
pub use messenger::{Decision, Messenger, PermissionMessage};
pub use notification_handler::NotificationInput;
pub use stop_handler::{StopEvent, StopInput};
