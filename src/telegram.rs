//! Telegram API helper functions.
//!
//! Provides utilities for creating inline keyboards and parsing callback data.

use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// User decision on a permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
    AlwaysAllow,
}

impl Decision {
    /// Convert decision to Claude Code hook behavior string.
    pub fn to_behavior(self) -> &'static str {
        match self {
            Decision::Allow | Decision::AlwaysAllow => "allow",
            Decision::Deny => "deny",
        }
    }
}

/// Create an inline keyboard for permission requests.
///
/// Returns a keyboard with Allow, Deny, and Always Allow buttons.
pub fn create_permission_keyboard(request_id: &str, tool_name: &str) -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![
            InlineKeyboardButton::callback("✅ Allow", format!("{}:allow", request_id)),
            InlineKeyboardButton::callback("❌ Deny", format!("{}:deny", request_id)),
        ],
        vec![InlineKeyboardButton::callback(
            "🔓 Always Allow",
            format!("{}:always_allow:{}", request_id, tool_name),
        )],
    ];

    InlineKeyboardMarkup::new(buttons)
}

/// Parsed callback data from a button press.
#[derive(Debug, Clone)]
pub struct CallbackData {
    pub request_id: String,
    pub decision: Decision,
    pub tool_name: Option<String>,
}

/// Parse callback data from a button press.
///
/// Format: `{request_id}:{decision}` or `{request_id}:{decision}:{tool_name}`
pub fn parse_callback_data(data: &str) -> Option<CallbackData> {
    let parts: Vec<&str> = data.split(':').collect();

    if parts.len() < 2 {
        return None;
    }

    let request_id = parts[0].to_string();
    let decision = match parts[1] {
        "allow" => Decision::Allow,
        "deny" => Decision::Deny,
        "always_allow" => Decision::AlwaysAllow,
        _ => return None,
    };

    let tool_name = if parts.len() >= 3 {
        Some(parts[2].to_string())
    } else {
        None
    };

    Some(CallbackData {
        request_id,
        decision,
        tool_name,
    })
}

/// Escape special characters for Telegram MarkdownV2 format.
pub fn escape_markdown(text: &str) -> String {
    let special_chars = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
    ];
    let mut result = String::with_capacity(text.len() * 2);

    for c in text.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callback_data_allow() {
        let data = parse_callback_data("abc123:allow").unwrap();
        assert_eq!(data.request_id, "abc123");
        assert_eq!(data.decision, Decision::Allow);
        assert!(data.tool_name.is_none());
    }

    #[test]
    fn test_parse_callback_data_deny() {
        let data = parse_callback_data("abc123:deny").unwrap();
        assert_eq!(data.request_id, "abc123");
        assert_eq!(data.decision, Decision::Deny);
    }

    #[test]
    fn test_parse_callback_data_always_allow() {
        let data = parse_callback_data("abc123:always_allow:Bash").unwrap();
        assert_eq!(data.request_id, "abc123");
        assert_eq!(data.decision, Decision::AlwaysAllow);
        assert_eq!(data.tool_name, Some("Bash".to_string()));
    }

    #[test]
    fn test_parse_callback_data_invalid() {
        assert!(parse_callback_data("invalid").is_none());
        assert!(parse_callback_data("abc123:unknown").is_none());
    }

    #[test]
    fn test_decision_to_behavior() {
        assert_eq!(Decision::Allow.to_behavior(), "allow");
        assert_eq!(Decision::Deny.to_behavior(), "deny");
        assert_eq!(Decision::AlwaysAllow.to_behavior(), "allow");
    }

    #[test]
    fn test_escape_markdown() {
        assert_eq!(escape_markdown("hello"), "hello");
        assert_eq!(escape_markdown("hello_world"), "hello\\_world");
        assert_eq!(escape_markdown("test.txt"), "test\\.txt");
        assert_eq!(escape_markdown("*bold*"), "\\*bold\\*");
    }

    #[test]
    fn test_create_permission_keyboard() {
        let keyboard = create_permission_keyboard("abc123", "Bash");
        assert_eq!(keyboard.inline_keyboard.len(), 2);
        assert_eq!(keyboard.inline_keyboard[0].len(), 2); // Allow, Deny
        assert_eq!(keyboard.inline_keyboard[1].len(), 1); // Always Allow
    }
}
