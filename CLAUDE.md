# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Claude Code hooks integration with messaging platforms (Telegram, Discord, Signal) written in Rust (~4MB Telegram-only, ~8MB with Discord, ~30MB with Signal).

Features:
- Intercept Claude Code permission requests via hooks
- Send notifications to users via Telegram (inline keyboards), Discord (buttons), or Signal (text-based)
- Receive user decisions (approve/deny/always allow) through messaging platforms
- Respond back to Claude Code with the user's decision
- Job completion notifications via Stop hooks
- Generic event handler for all Claude Code hook events (SessionStart, PostToolUseFailure, TaskCompleted, etc.)
- Configurable event filters to control notification volume
- Discord support via optional `--features discord` build flag (MIT/Apache 2.0)
- Signal support via optional `--features signal` build flag (AGPL-3.0 licensed)

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Claude Code    │────▶│  Hook Handler    │────▶│  Telegram Bot   │
│  (Permission    │     │  (Rust)          │     │  API            │
│   Request)      │     │                  │     │                 │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                │                        │
                                │                        ▼
                                │                 ┌─────────────────┐
                                │                 │  User Device    │
                                │                 │  (Telegram App) │
                                │                 └─────────────────┘
                                │                        │
                                ▼                        ▼
                        ┌──────────────────────────────────────────┐
                        │  Decision Handler (receives callback)    │
                        │  Returns: allow/deny to Claude Code      │
                        └──────────────────────────────────────────┘
```

## Package Structure

```
src/
├── main.rs               # Entry point + tokio runtime
├── lib.rs                # Library root
├── cli.rs                # Clap subcommands (hook, stop, notify, event, bot, etc.)
├── config.rs             # JSON/env config loading (supports event_filters)
├── util.rs               # Shared utilities (read_stdin, project_name_from_cwd)
├── always_allow.rs       # Tool whitelist persistence
├── hook_handler.rs       # PermissionRequest handler (uses Messenger trait)
├── stop_handler.rs       # Stop hook - job completion notifications
├── notification_handler.rs # Notification hook handler
├── event_handler.rs      # Generic handler for all other hook events
├── bot.rs                # Long-running Telegram bot
├── telegram.rs           # Legacy re-exports for backward compatibility
├── error.rs              # Error types
└── messenger/            # Messenger abstraction layer
    ├── mod.rs            # Messenger trait + resolve_messenger()
    ├── types.rs          # Decision enum, PermissionMessage struct
    ├── telegram.rs       # Telegram implementation (inline keyboards)
    ├── discord.rs        # Discord implementation (buttons, requires --features discord)
    └── signal.rs         # Signal implementation (text-based, requires --features signal)
```

## Claude Code Hook Integration

Claude Code hooks are configured in `~/.claude/settings.json` or project's `.claude/settings.json`:
```json
{
  "hooks": {
    "PermissionRequest": [
      {
        "matcher": { "tools": ["Bash", "Edit", "Write"] },
        "hooks": [{ "type": "command", "command": "claude-code-telegram hook" }]
      }
    ],
    "Stop": [
      {
        "matcher": {},
        "hooks": [{ "type": "command", "command": "claude-code-telegram stop" }]
      }
    ],
    "Notification": [
      {
        "matcher": {},
        "hooks": [{ "type": "command", "command": "claude-code-telegram notify" }]
      }
    ],
    "SessionStart": [
      {
        "matcher": {},
        "hooks": [{ "type": "command", "command": "claude-code-telegram event SessionStart" }]
      }
    ],
    "PostToolUseFailure": [
      {
        "matcher": {},
        "hooks": [{ "type": "command", "command": "claude-code-telegram event PostToolUseFailure" }]
      }
    ],
    "TaskCompleted": [
      {
        "matcher": {},
        "hooks": [{ "type": "command", "command": "claude-code-telegram event TaskCompleted" }]
      }
    ]
  }
}
```

Use `claude-code-telegram hooks-config` to generate a full hooks configuration for all supported events.

The hook script receives JSON via stdin with the permission request details and must output a JSON response.

## Development Commands

```bash
# Development build
cargo build

# Release build (~4MB)
cargo build --release

# Build with Discord support (~8MB)
cargo build --release --features discord

# Build with Signal support (~30MB)
cargo build --release --features signal

# Run tests
cargo test

# Run tests with Discord feature
cargo test --features discord

# Run tests with Signal feature
cargo test --features signal

# Run clippy lints
cargo clippy --all-targets -- -D warnings

# Run clippy with Discord feature
cargo clippy --all-targets --features discord -- -D warnings

# Run clippy with Signal feature
cargo clippy --all-targets --features signal -- -D warnings

# Format code
cargo fmt

# Run CLI commands
./target/release/claude-code-telegram hook                          # PermissionRequest handler
./target/release/claude-code-telegram stop                          # Stop handler
./target/release/claude-code-telegram notify                        # Notification handler
./target/release/claude-code-telegram event SessionStart            # Generic event handler
./target/release/claude-code-telegram event PostToolUseFailure      # Generic event handler
./target/release/claude-code-telegram bot                           # Telegram bot
./target/release/claude-code-telegram status                        # Show config status
./target/release/claude-code-telegram hooks-config                  # Print hooks config JSON
./target/release/claude-code-telegram signal-link                   # requires --features signal
```

## Configuration

The hook loads configuration in this priority order:
1. JSON file at `~/.claude/telegram_hook.json` (recommended)
2. Environment variables (fallback)

### JSON Configuration (Recommended)

**Legacy format** (Telegram only):
```json
{
  "telegram_bot_token": "your_bot_token",
  "telegram_chat_id": "your_chat_id"
}
```

**New multi-messenger format** (Telegram + Discord + Signal):
```json
{
  "messengers": {
    "telegram": {
      "enabled": true,
      "bot_token": "your_bot_token",
      "chat_id": "your_chat_id"
    },
    "discord": {
      "enabled": true,
      "bot_token": "your_discord_bot_token",
      "user_id": "your_discord_user_id"
    },
    "signal": {
      "enabled": true,
      "phone_number": "+1234567890",
      "device_name": "claude-code",
      "data_path": "~/.claude/signal_data"
    }
  },
  "preferences": {
    "primary_messenger": "telegram",
    "timeout_seconds": 300,
    "event_filters": {
      "SessionStart": true,
      "SessionEnd": true,
      "PostToolUseFailure": true,
      "TaskCompleted": true,
      "TeammateIdle": true,
      "SubagentStart": false,
      "PreToolUse": false
    }
  }
}
```

Both formats are supported - the legacy format is auto-detected and converted.

### Event Filters

The `event_filters` preference controls which hook events send notifications.
Events enabled by default: `SessionStart`, `SessionEnd`, `PostToolUseFailure`, `TaskCompleted`, `TeammateIdle`.
All other events are disabled by default but can be enabled via config.

### Environment Variables (Fallback)

Store in `~/.claude/.env` (never commit):
- `TELEGRAM_BOT_TOKEN`: Bot token from @BotFather
- `TELEGRAM_CHAT_ID`: Target chat ID for notifications

### Data Files

- `~/.claude/always_allow.json`: Stores always-allow tool preferences

## Dependencies (Cargo.toml)

Core dependencies:
- `teloxide`: Telegram Bot API
- `tokio`: Async runtime
- `serde` + `serde_json`: JSON serialization
- `clap`: CLI with subcommands
- `directories`: Cross-platform config paths
- `dotenvy`: .env file loading
- `thiserror` / `anyhow`: Error handling
- `uuid`: Request ID generation
- `hostname`: System hostname
- `async-trait`: Async trait support

Discord feature dependencies (optional, MIT/Apache 2.0):
- `serenity`: Discord Bot API

Signal feature dependencies (optional, AGPL-3.0):
- `presage`: Signal protocol implementation
- `presage-store-sqlite`: SQLite storage for Signal data
- `qrcode`: QR code generation for device linking
- `futures-util`, `futures-channel`: Async utilities

## Archived Python Version

The original Python implementation is preserved in `archives/` for reference. It used PEX/scie-jump for self-contained binaries but resulted in ~50MB files.

## Commit & Push guidance
Before making commit, make sure to always check linter (`cargo fmt`).
