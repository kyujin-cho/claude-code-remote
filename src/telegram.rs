//! Telegram API helper functions.
//!
//! This module re-exports types from the messenger module for backward compatibility.
//! New code should use `crate::messenger::telegram` directly.

// Re-export types for backward compatibility
pub use crate::messenger::telegram::escape_markdown;

// Note: create_permission_keyboard, parse_callback_data, and Decision are now
// in the messenger module as they are shared across platforms.
