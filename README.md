# Claude Code Remote

Claude Code hook & Telegram Bot to notify user about active CC permission request and receive the decision via Telegram bot.

## Features

- **Permission request notifications** via Telegram with inline keyboards
- **Always Allow** feature to auto-approve trusted tools
- **Job completion notifications** when Claude Code finishes
- **Multi-machine support** with hostname display
- **Small binary size**: ~4 MB (vs ~50 MB for the archived Python version)

## Installation

### Option A: Download Pre-built Binary (Recommended)

Download the latest release for your platform:

```bash
# macOS (Apple Silicon)
curl -L -o claude-code-telegram \
  https://github.com/kyujin-cho/claude-code-remote/releases/latest/download/claude-code-telegram-macos-aarch64
chmod +x claude-code-telegram
sudo mv claude-code-telegram /usr/local/bin/

# macOS (Intel)
curl -L -o claude-code-telegram \
  https://github.com/kyujin-cho/claude-code-remote/releases/latest/download/claude-code-telegram-macos-x86_64
chmod +x claude-code-telegram
sudo mv claude-code-telegram /usr/local/bin/

# Linux (x86_64)
curl -L -o claude-code-telegram \
  https://github.com/kyujin-cho/claude-code-remote/releases/latest/download/claude-code-telegram-linux-x86_64
chmod +x claude-code-telegram
sudo mv claude-code-telegram /usr/local/bin/
```

### Option B: Build from Source

```bash
# Requires Rust toolchain (https://rustup.rs)
cargo build --release
sudo cp target/release/claude-code-telegram /usr/local/bin/
```

## Setup

### 1. Create a Telegram Bot

1. Message [@BotFather](https://t.me/botfather) on Telegram
2. Send `/newbot` and follow the prompts
3. Save the bot token

### 2. Get Your Chat ID

1. Message [@userinfobot](https://t.me/userinfobot) on Telegram
2. It will reply with your chat ID

### 3. Configure Credentials

Create `~/.claude/telegram_hook.json`:

```json
{
  "telegram_bot_token": "your_bot_token_here",
  "telegram_chat_id": "your_chat_id_here"
}
```

### 4. Configure Claude Code Hooks

Add to your `~/.claude/settings.json` or project `.claude/settings.json`:

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

The `Stop` hook is optional - add it if you want job completion notifications.

## Usage

When Claude Code attempts to use a matched tool (Bash, Edit, Write), you'll receive a Telegram notification with:

- Host name (identifies which machine the request came from)
- Tool name
- Command/file details
- Allow, Deny, and Always Allow buttons

Tap a button to respond. The decision is sent back to Claude Code.

### Always Allow Feature

When you click "Always Allow" for a tool, future requests for that tool will be automatically approved. You'll still receive a notification showing what was auto-approved.

Preferences are stored in `~/.claude/always_allow.json`:

```json
{
  "tools": ["Bash", "Edit"]
}
```

To reset preferences, delete or edit this file.

## CLI Commands

```bash
# Permission request hook handler (used by Claude Code PermissionRequest hooks)
claude-code-telegram hook

# Job completion hook handler (used by Claude Code Stop hooks)
claude-code-telegram stop

# Run the Telegram bot (for /start, /help, /status commands)
claude-code-telegram bot

# Show help
claude-code-telegram --help
```

## Development

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run clippy lints
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt
```

## Cross-Compilation Targets

- `x86_64-unknown-linux-musl` (Linux x86_64, static)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)

## Archived Python Version

The original Python implementation is preserved in the `archives/` directory for reference. It used PEX/scie-jump to create self-contained binaries but resulted in ~50 MB files. The Rust rewrite achieves the same functionality with ~4 MB binaries.
