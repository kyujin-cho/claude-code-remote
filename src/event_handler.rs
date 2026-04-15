//! Generic event handler for all Claude Code hook events.
//!
//! Handles notification-only hook events (SessionStart, PostToolUseFailure, etc.)
//! via a single unified entry point. Events are formatted with per-event formatters
//! for high-value events and a generic fallback for others.

use crate::config::Config;
use crate::error::HookError;
use crate::messenger;
use crate::util;
use serde::Deserialize;
use serde_json::Value;

/// Generic event input that captures all fields from any hook event.
#[derive(Debug, Deserialize)]
pub struct GenericEventInput {
    #[serde(default)]
    #[allow(dead_code)]
    pub session_id: String,
    #[serde(default)]
    pub cwd: String,
    /// All additional event-specific fields captured via flatten
    #[serde(flatten)]
    pub extra: Value,
}

/// Format an event notification message based on the event name.
fn format_event(event_name: &str, input: &GenericEventInput, hostname: &str) -> String {
    let project = util::project_name_from_cwd(&input.cwd);

    match event_name {
        "SessionStart" => format_session_start(input, hostname, &project),
        "SessionEnd" => format_session_end(input, hostname, &project),
        "PostToolUseFailure" => format_post_tool_use_failure(input, hostname, &project),
        "TaskCompleted" => format_task_completed(input, hostname, &project),
        "TeammateIdle" => format_teammate_idle(input, hostname, &project),
        "SubagentStart" => format_subagent(input, hostname, &project, true),
        "SubagentStop" => format_subagent(input, hostname, &project, false),
        "UserPromptSubmit" => format_user_prompt_submit(input, hostname, &project),
        "PreToolUse" => format_tool_use(input, hostname, &project, "Pre"),
        "PostToolUse" => format_tool_use(input, hostname, &project, "Post"),
        _ => format_generic(event_name, input, hostname, &project),
    }
}

fn format_session_start(input: &GenericEventInput, hostname: &str, project: &str) -> String {
    let model = input
        .extra
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let permission_mode = input
        .extra
        .get("permission_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    let mut lines = vec![
        "Session Started".to_string(),
        format!("Host: {}", hostname),
        format!("Project: {}", project),
        format!("Model: {}", model),
    ];

    if permission_mode != "default" {
        lines.push(format!("Mode: {}", permission_mode));
    }

    lines.join("\n")
}

fn format_session_end(input: &GenericEventInput, hostname: &str, project: &str) -> String {
    let transcript = input
        .extra
        .get("transcript_path")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut lines = vec![
        "Session Ended".to_string(),
        format!("Host: {}", hostname),
        format!("Project: {}", project),
    ];

    if !transcript.is_empty() {
        lines.push(format!("Transcript: {}", transcript));
    }

    lines.join("\n")
}

fn format_post_tool_use_failure(
    input: &GenericEventInput,
    hostname: &str,
    project: &str,
) -> String {
    let tool_name = input
        .extra
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let error = input
        .extra
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown error");

    let truncated_error: String = error.chars().take(500).collect();
    let error_display = if error.len() > 500 {
        format!("{}...", truncated_error)
    } else {
        truncated_error
    };

    [
        "Tool Execution Failed".to_string(),
        format!("Host: {}", hostname),
        format!("Project: {}", project),
        format!("Tool: {}", tool_name),
        String::new(),
        format!("Error:\n{}", error_display),
    ]
    .join("\n")
}

fn format_task_completed(input: &GenericEventInput, hostname: &str, project: &str) -> String {
    let task_result = input
        .extra
        .get("task_result")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut lines = vec![
        "Task Completed".to_string(),
        format!("Host: {}", hostname),
        format!("Project: {}", project),
    ];

    if !task_result.is_empty() {
        let truncated: String = task_result.chars().take(300).collect();
        let display = if task_result.len() > 300 {
            format!("{}...", truncated)
        } else {
            truncated
        };
        lines.push(String::new());
        lines.push(format!("Result:\n{}", display));
    }

    lines.join("\n")
}

fn format_teammate_idle(input: &GenericEventInput, hostname: &str, project: &str) -> String {
    let teammate_id = input
        .extra
        .get("teammate_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    [
        "Teammate Idle - Waiting for Input".to_string(),
        format!("Host: {}", hostname),
        format!("Project: {}", project),
        format!("Teammate: {}", teammate_id),
    ]
    .join("\n")
}

fn format_subagent(
    input: &GenericEventInput,
    hostname: &str,
    project: &str,
    is_start: bool,
) -> String {
    let agent_type = input
        .extra
        .get("agent_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let action = if is_start { "Started" } else { "Stopped" };

    [
        format!("Subagent {}", action),
        format!("Host: {}", hostname),
        format!("Project: {}", project),
        format!("Type: {}", agent_type),
    ]
    .join("\n")
}

fn format_user_prompt_submit(input: &GenericEventInput, hostname: &str, project: &str) -> String {
    let prompt = input
        .extra
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut lines = vec![
        "User Prompt Submitted".to_string(),
        format!("Host: {}", hostname),
        format!("Project: {}", project),
    ];

    if !prompt.is_empty() {
        let truncated: String = prompt.chars().take(200).collect();
        let display = if prompt.len() > 200 {
            format!("{}...", truncated)
        } else {
            truncated
        };
        lines.push(String::new());
        lines.push(display);
    }

    lines.join("\n")
}

fn format_tool_use(
    input: &GenericEventInput,
    hostname: &str,
    project: &str,
    phase: &str,
) -> String {
    let tool_name = input
        .extra
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    [
        format!("{} Tool Use", phase),
        format!("Host: {}", hostname),
        format!("Project: {}", project),
        format!("Tool: {}", tool_name),
    ]
    .join("\n")
}

fn format_generic(
    event_name: &str,
    input: &GenericEventInput,
    hostname: &str,
    project: &str,
) -> String {
    let mut lines = vec![
        format!("Hook Event: {}", event_name),
        format!("Host: {}", hostname),
    ];

    if !input.cwd.is_empty() {
        lines.push(format!("Project: {}", project));
    }

    lines.join("\n")
}

/// Main entry point for generic event handling.
pub async fn run(event_name: &str) -> Result<(), HookError> {
    let input_str = util::read_stdin()?;
    let input: GenericEventInput = serde_json::from_str(&input_str)?;

    let config = Config::load(None)?;

    // Check if event is enabled
    if !config.is_event_enabled(event_name) {
        return Ok(());
    }

    let resolved = match messenger::resolve_messenger(&config) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };

    let text = format_event(event_name, &input, &config.hostname);
    resolved.send_notification(&text).await?;

    Ok(())
}

/// Print recommended hooks configuration for all supported events.
pub fn print_hooks_config() {
    let config = serde_json::json!({
        "hooks": {
            "PermissionRequest": [
                {
                    "matcher": {
                        "tools": ["Bash", "Edit", "Write", "NotebookEdit"]
                    },
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram hook"
                    }]
                }
            ],
            "Stop": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram stop"
                    }]
                }
            ],
            "Notification": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram notify"
                    }]
                }
            ],
            "SessionStart": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event SessionStart"
                    }]
                }
            ],
            "SessionEnd": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event SessionEnd"
                    }]
                }
            ],
            "PostToolUseFailure": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event PostToolUseFailure"
                    }]
                }
            ],
            "TaskCompleted": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event TaskCompleted"
                    }]
                }
            ],
            "TeammateIdle": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event TeammateIdle"
                    }]
                }
            ],
            "SubagentStart": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event SubagentStart"
                    }]
                }
            ],
            "SubagentStop": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event SubagentStop"
                    }]
                }
            ],
            "UserPromptSubmit": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event UserPromptSubmit"
                    }]
                }
            ],
            "PreToolUse": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event PreToolUse"
                    }]
                }
            ],
            "PostToolUse": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event PostToolUse"
                    }]
                }
            ],
            "PreCompact": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event PreCompact"
                    }]
                }
            ],
            "PostCompact": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event PostCompact"
                    }]
                }
            ],
            "WorktreeCreate": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event WorktreeCreate"
                    }]
                }
            ],
            "WorktreeRemove": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event WorktreeRemove"
                    }]
                }
            ],
            "ConfigChange": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event ConfigChange"
                    }]
                }
            ],
            "InstructionsLoaded": [
                {
                    "matcher": {},
                    "hooks": [{
                        "type": "command",
                        "command": "claude-code-telegram event InstructionsLoaded"
                    }]
                }
            ]
        }
    });

    println!("{}", serde_json::to_string_pretty(&config).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(cwd: &str, extra: Value) -> GenericEventInput {
        GenericEventInput {
            session_id: "test-session".to_string(),
            cwd: cwd.to_string(),
            extra,
        }
    }

    #[test]
    fn test_format_session_start() {
        let input = make_input(
            "/home/user/my-project",
            serde_json::json!({
                "model": "opus",
                "permission_mode": "plan"
            }),
        );

        let result = format_event("SessionStart", &input, "test-host");
        assert!(result.contains("Session Started"));
        assert!(result.contains("test-host"));
        assert!(result.contains("my-project"));
        assert!(result.contains("opus"));
        assert!(result.contains("plan"));
    }

    #[test]
    fn test_format_session_start_default_mode() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({"model": "sonnet", "permission_mode": "default"}),
        );

        let result = format_event("SessionStart", &input, "host");
        assert!(!result.contains("Mode:"));
    }

    #[test]
    fn test_format_session_end() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({"transcript_path": "/tmp/transcript.jsonl"}),
        );

        let result = format_event("SessionEnd", &input, "host");
        assert!(result.contains("Session Ended"));
        assert!(result.contains("transcript.jsonl"));
    }

    #[test]
    fn test_format_post_tool_use_failure() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({
                "tool_name": "Bash",
                "error": "permission denied"
            }),
        );

        let result = format_event("PostToolUseFailure", &input, "host");
        assert!(result.contains("Tool Execution Failed"));
        assert!(result.contains("Bash"));
        assert!(result.contains("permission denied"));
    }

    #[test]
    fn test_format_post_tool_use_failure_truncates_error() {
        let long_error = "x".repeat(600);
        let input = make_input(
            "/tmp",
            serde_json::json!({"tool_name": "Bash", "error": long_error}),
        );

        let result = format_event("PostToolUseFailure", &input, "host");
        assert!(result.contains("..."));
        assert!(result.len() < 700);
    }

    #[test]
    fn test_format_task_completed() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({"task_result": "All tests passed"}),
        );

        let result = format_event("TaskCompleted", &input, "host");
        assert!(result.contains("Task Completed"));
        assert!(result.contains("All tests passed"));
    }

    #[test]
    fn test_format_teammate_idle() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({"teammate_id": "agent-1"}),
        );

        let result = format_event("TeammateIdle", &input, "host");
        assert!(result.contains("Teammate Idle"));
        assert!(result.contains("agent-1"));
    }

    #[test]
    fn test_format_subagent_start() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({"agent_type": "code-review"}),
        );

        let result = format_event("SubagentStart", &input, "host");
        assert!(result.contains("Subagent Started"));
        assert!(result.contains("code-review"));
    }

    #[test]
    fn test_format_subagent_stop() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({"agent_type": "code-review"}),
        );

        let result = format_event("SubagentStop", &input, "host");
        assert!(result.contains("Subagent Stopped"));
    }

    #[test]
    fn test_format_user_prompt_submit() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({"prompt": "Fix the login bug"}),
        );

        let result = format_event("UserPromptSubmit", &input, "host");
        assert!(result.contains("User Prompt Submitted"));
        assert!(result.contains("Fix the login bug"));
    }

    #[test]
    fn test_format_generic_unknown_event() {
        let input = make_input("/home/user/project", serde_json::json!({}));

        let result = format_event("SomeNewEvent", &input, "host");
        assert!(result.contains("Hook Event: SomeNewEvent"));
        assert!(result.contains("host"));
    }

    #[test]
    fn test_format_tool_use() {
        let input = make_input(
            "/home/user/project",
            serde_json::json!({"tool_name": "Edit"}),
        );

        let result = format_event("PreToolUse", &input, "host");
        assert!(result.contains("Pre Tool Use"));
        assert!(result.contains("Edit"));

        let result = format_event("PostToolUse", &input, "host");
        assert!(result.contains("Post Tool Use"));
    }
}
