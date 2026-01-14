# Claude Code Remote

Claude Code hook & Telegram Bot to notify user about active CC permission request and receive the decision via Telegram bot.

## Features

- Intercepts Claude Code permission requests via hooks
- Sends notifications to Telegram with tool details
- Inline keyboard buttons for Allow/Deny/Always Allow decisions
- **Always Allow**: Remember tool preferences for automatic approval
- **Job Completion Notifications**: Get notified when Claude Code finishes a task
- Returns decisions back to Claude Code

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

The binary is self-contained with Python bundled - no Python installation required.

### Option B: Install from Source

```bash
pip install -e .
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

**Option A: JSON Configuration File (Recommended)**

Create `~/.claude/telegram_hook.json`:
```json
{
  "telegram_bot_token": "your_bot_token_here",
  "telegram_chat_id": "your_chat_id_here"
}
```

**Option B: Environment Variables**

Create `~/.claude/.env` (or set environment variables):
```
TELEGRAM_BOT_TOKEN=your_bot_token_here
TELEGRAM_CHAT_ID=your_chat_id_here
```

The hook checks for JSON config first, then falls back to environment variables.

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

After installation, a single `claude-code-telegram` command is available with subcommands:

```bash
# Permission request hook handler (used by Claude Code PermissionRequest hooks)
claude-code-telegram hook

# Job completion hook handler (used by Claude Code Stop hooks)
claude-code-telegram stop

# Run the Telegram bot (for /start, /help commands)
claude-code-telegram bot

# Show help
claude-code-telegram --help
```

## Development

```bash
# Install development dependencies (using uv)
uv sync --all-extras

# Run tests
make test

# Run linting
make lint

# Format code
make format
```

### Building Self-Executable Binary

The project uses [scie-jump](https://github.com/a-scie/jump) via PEX to create self-contained executables with Python bundled.

```bash
# Install pex
pip install pex

# Build binary (eager mode - ~50MB, works offline)
make build-scie

# Build binary (lazy mode - ~5MB, fetches Python on first run)
make build-scie-lazy

# Output: dist/claude-code-telegram
```

Or use the build script directly:
```bash
./scripts/build_scie.sh eager  # or 'lazy'
```

## Key Technologies

- PermissionRequest (from Claude Code hook)
- InlineKeyboardButton (from Telegram Bot)
- [scie-jump](https://github.com/a-scie/jump) + [PEX](https://github.com/pex-tool/pex) for self-contained binaries
