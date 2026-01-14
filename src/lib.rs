//! Claude Code Telegram Bot library.
//!
//! This library provides the core functionality for the Claude Code Telegram integration.

pub mod always_allow;
pub mod bot;
pub mod cli;
pub mod config;
pub mod error;
pub mod hook_handler;
pub mod stop_handler;
pub mod telegram;

// Re-export commonly used types
pub use always_allow::AlwaysAllowManager;
pub use config::Config;
pub use hook_handler::{HookInput, HookOutput, PermissionRequest};
pub use stop_handler::{StopEvent, StopInput};
pub use telegram::Decision;
