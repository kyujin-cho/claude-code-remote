# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Claude Code hooks integration with Telegram Bot written in Rust (~4MB binary).

Features:
- Intercept Claude Code permission requests via hooks
- Send notifications to users via Telegram (includes hostname for multi-machine setups)
- Receive user decisions (approve/deny/always allow) through Telegram inline keyboards
- Respond back to Claude Code with the user's decision
- Job completion notifications via Stop hooks

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
├── main.rs           # Entry point + tokio runtime
├── lib.rs            # Library root
├── cli.rs            # Clap subcommands (hook, stop, bot)
├── config.rs         # JSON/env config loading
├── always_allow.rs   # Tool whitelist persistence
├── hook_handler.rs   # Permission request handler
├── stop_handler.rs   # Job completion notifications
├── bot.rs            # Long-running Telegram bot
├── telegram.rs       # Keyboard/callback helpers
└── error.rs          # Error types
```

## Claude Code Hook Integration

Claude Code hooks are configured in `~/.claude/settings.json` or project's `.claude/settings.json`:
```json
{
  "hooks": {
    "PermissionRequest": [
      {
        "matcher": {
          "tools": ["Bash", "Edit", "Write"]
        },
        "hooks": [
          {
            "type": "command",
            "command": "claude-code-telegram hook"
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": {},
        "hooks": [
          {
            "type": "command",
            "command": "claude-code-telegram stop"
          }
        ]
      }
    ]
  }
}
```

The hook script receives JSON via stdin with the permission request details and must output a JSON response.

## Development Commands

```bash
# Development build
cargo build

# Release build (~4MB)
cargo build --release

# Run tests
cargo test

# Run clippy lints
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt

# Run CLI commands
./target/release/claude-code-telegram hook
./target/release/claude-code-telegram stop
./target/release/claude-code-telegram bot
```

## Configuration

The hook loads configuration in this priority order:
1. JSON file at `~/.claude/telegram_hook.json` (recommended)
2. Environment variables (fallback)

### JSON Configuration (Recommended)

Create `~/.claude/telegram_hook.json`:
```json
{
  "telegram_bot_token": "your_bot_token",
  "telegram_chat_id": "your_chat_id"
}
```

### Environment Variables (Fallback)

Store in `~/.claude/.env` (never commit):
- `TELEGRAM_BOT_TOKEN`: Bot token from @BotFather
- `TELEGRAM_CHAT_ID`: Target chat ID for notifications

### Data Files

- `~/.claude/always_allow.json`: Stores always-allow tool preferences

## Dependencies (Cargo.toml)

- `teloxide`: Telegram Bot API
- `tokio`: Async runtime
- `serde` + `serde_json`: JSON serialization
- `clap`: CLI with subcommands
- `directories`: Cross-platform config paths
- `dotenvy`: .env file loading
- `thiserror` / `anyhow`: Error handling
- `uuid`: Request ID generation
- `hostname`: System hostname

## Archived Python Version

The original Python implementation is preserved in `archives/` for reference. It used PEX/scie-jump for self-contained binaries but resulted in ~50MB files.
