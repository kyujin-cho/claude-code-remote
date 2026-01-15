# Claude Code Remote

Claude Code hook & messaging integration to notify you about permission requests and receive decisions remotely.

## Features

- **Permission request notifications** via Telegram, Discord (with buttons), or Signal (text-based)
- **Always Allow** feature to auto-approve trusted tools
- **Job completion notifications** when Claude Code finishes
- **Multi-machine support** with hostname display
- **Small binary size**: ~4 MB Telegram-only, ~8 MB with Discord, ~30 MB with Signal

## Installation

### Quick Install (Recommended)

Run the interactive installer:

```bash
curl -fsSL https://raw.githubusercontent.com/kyujin-cho/claude-code-remote/main/install.sh | bash
```

The installer will:
1. Detect your platform and download the appropriate binary
2. Prompt you to configure your preferred messenger (Telegram/Discord/Signal)
3. Set up Claude Code hooks automatically

**Installer options:**
```bash
# Skip configuration (binary only)
curl -fsSL ... | bash -s -- --skip-config

# Install to custom directory
curl -fsSL ... | bash -s -- --install-dir ~/.local/bin

# Install specific version
curl -fsSL ... | bash -s -- --version v1.0.0
```

### Manual Download

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

### Build from Source

Requires [Rust toolchain](https://rustup.rs).

```bash
# Telegram only (~4 MB)
cargo build --release
sudo cp target/release/claude-code-telegram /usr/local/bin/

# With Discord support (~8 MB)
cargo build --release --features discord
sudo cp target/release/claude-code-telegram /usr/local/bin/

# With Signal support (~30 MB, AGPL-3.0 license)
cargo build --release --features signal
sudo cp target/release/claude-code-telegram /usr/local/bin/
```

**Note:** Signal integration uses [presage](https://github.com/whisperfish/presage) which is licensed under AGPL-3.0. Building with `--features signal` makes the resulting binary subject to AGPL-3.0 licensing requirements.

## Setup

Choose **one or more** messengers to configure. You only need to set up the messenger(s) you want to use.

### Option A: Telegram Setup

1. Message [@BotFather](https://t.me/botfather) on Telegram
2. Send `/newbot` and follow the prompts
3. Save the bot token
4. Message [@userinfobot](https://t.me/userinfobot) to get your chat ID

### Option B: Discord Setup

Requires building with `--features discord` or using a Discord-enabled binary.

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application → Bot section → Reset Token → Copy token
3. Enable "Message Content Intent" in Bot settings
4. OAuth2 → URL Generator → Select `bot` scope → `Send Messages` permission
5. Open the generated URL to invite the bot
6. Get your User ID: Enable Developer Mode → Right-click your name → Copy ID

### Option C: Signal Setup

Requires building with `--features signal` or using a Signal-enabled binary.

1. Create data directory and link device:
   ```bash
   mkdir -p ~/.claude/signal_data
   claude-code-telegram signal-link --device-name "claude-code"
   ```
2. Open Signal on your phone: Settings > Linked Devices > Link New Device
3. Scan the QR code displayed in the terminal

**Signal Limitations:**
- No inline keyboard support - you must reply with text commands
- Reply format: `ALLOW <request_id>`, `DENY <request_id>`, or `ALWAYS <request_id>`
- Example: `ALLOW abc123`

---

### Configure Credentials

Create `~/.claude/hook_config.json` with your chosen messenger(s):

**Telegram only:**
```json
{
  "messengers": {
    "telegram": {
      "enabled": true,
      "bot_token": "your_bot_token",
      "chat_id": "your_chat_id"
    }
  },
  "preferences": {
    "primary_messenger": "telegram",
    "timeout_seconds": 300
  }
}
```

**Discord only:**
```json
{
  "messengers": {
    "discord": {
      "enabled": true,
      "bot_token": "your_discord_bot_token",
      "user_id": "your_discord_user_id"
    }
  },
  "preferences": {
    "primary_messenger": "discord",
    "timeout_seconds": 300
  }
}
```

**Signal only:**
```json
{
  "messengers": {
    "signal": {
      "enabled": true,
      "phone_number": "+1234567890",
      "device_name": "claude-code",
      "data_path": "~/.claude/signal_data"
    }
  },
  "preferences": {
    "primary_messenger": "signal",
    "timeout_seconds": 300
  }
}
```

**Multiple messengers (Discord primary, Telegram fallback):**
```json
{
  "messengers": {
    "telegram": {
      "enabled": true,
      "bot_token": "your_telegram_bot_token",
      "chat_id": "your_telegram_chat_id"
    },
    "discord": {
      "enabled": true,
      "bot_token": "your_discord_bot_token",
      "user_id": "your_discord_user_id"
    }
  },
  "preferences": {
    "primary_messenger": "discord",
    "timeout_seconds": 300
  }
}
```

> **Legacy format:** The old `~/.claude/telegram_hook.json` format with `telegram_bot_token` and `telegram_chat_id` fields is still supported for backward compatibility.

### Configure Claude Code Hooks

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
    ],
    "Notification": [
      {
        "matcher": {},
        "hooks": [
          {
            "type": "command",
            "command": "claude-code-telegram notify"
          }
        ]
      }
    ]
  }
}
```

**Hook types:**
- `PermissionRequest` - Required. Sends permission requests for tool usage.
- `Stop` - Optional. Sends job completion notifications with summary.
- `Notification` - Optional. Relays Claude Code notifications (idle prompts, etc.).

## Usage

When Claude Code attempts to use a matched tool (Bash, Edit, Write), you'll receive a notification via your configured messenger with:

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

# Notification relay handler (used by Claude Code Notification hooks)
claude-code-telegram notify

# Send a custom message to configured messengers
claude-code-telegram relay "Your message here"

# Run the Telegram bot (for /start, /help, /status commands)
claude-code-telegram bot

# Show configuration status
claude-code-telegram status

# Link Signal device (requires --features signal)
claude-code-telegram signal-link --device-name "my-device"

# Show help
claude-code-telegram --help
```

## Development

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Build with Discord support
cargo build --features discord

# Build with Signal support
cargo build --features signal

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
```

## Cross-Compilation Targets

- `x86_64-unknown-linux-musl` (Linux x86_64, static)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)

## Archived Python Version

The original Python implementation is preserved in the `archives/` directory for reference. It used PEX/scie-jump to create self-contained binaries but resulted in ~50 MB files. The Rust rewrite achieves the same functionality with ~4 MB binaries.
