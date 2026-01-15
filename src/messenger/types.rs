//! Shared types for messenger implementations.

use serde_json::Value;

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

/// Permission request message content.
#[derive(Debug, Clone)]
pub struct PermissionMessage {
    /// Unique request identifier (8-char UUID prefix)
    pub request_id: String,
    /// Tool name (e.g., "Bash", "Edit", "Write")
    pub tool_name: String,
    /// Hostname for multi-machine setups
    pub hostname: String,
    /// Tool input parameters
    pub tool_input: Value,
}

impl PermissionMessage {
    /// Create a new permission message.
    pub fn new(request_id: String, tool_name: String, hostname: String, tool_input: Value) -> Self {
        Self {
            request_id,
            tool_name,
            hostname,
            tool_input,
        }
    }
}
