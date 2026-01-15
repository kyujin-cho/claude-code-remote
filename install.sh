#!/usr/bin/env bash
#
# Claude Code Remote - Installation Script
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/kyujin-cho/claude-code-remote/main/install.sh | bash
#
# Or with options:
#   curl -fsSL ... | bash -s -- --help
#

set -e

# Preserve original stdout for user messages (fd 3)
exec 3>&1

# =============================================================================
# Configuration
# =============================================================================

REPO="kyujin-cho/claude-code-remote"
BINARY_NAME="claude-code-telegram"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
CLAUDE_DIR="${HOME}/.claude"
CONFIG_FILE="${CLAUDE_DIR}/hook_config.json"
LEGACY_CONFIG_FILE="${CLAUDE_DIR}/telegram_hook.json"
SETTINGS_FILE="${CLAUDE_DIR}/settings.json"

# =============================================================================
# Colors and Output
# =============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}==>${NC} ${BOLD}$1${NC}"
}

success() {
    echo -e "${GREEN}==>${NC} ${BOLD}$1${NC}"
}

warn() {
    echo -e "${YELLOW}Warning:${NC} $1"
}

error() {
    echo -e "${RED}Error:${NC} $1" >&2
}

# =============================================================================
# Helper Functions
# =============================================================================

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            OS="linux"
            ;;
        Darwin)
            OS="macos"
            ;;
        *)
            error "Unsupported operating system: $os"
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            exit 1
            ;;
    esac

    PLATFORM="${OS}-${ARCH}"
}

get_latest_version() {
    if command_exists curl; then
        curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command_exists wget; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

download_binary() {
    local version="$1"
    local url="https://github.com/${REPO}/releases/download/${version}/${BINARY_NAME}-${PLATFORM}"
    local tmp_file

    tmp_file="$(mktemp)"

    # Output to fd 3 (original stdout) to avoid contaminating return value
    echo -e "${BLUE}==>${NC} ${BOLD}Downloading ${BINARY_NAME} ${version} for ${PLATFORM}...${NC}" >&3

    if command_exists curl; then
        curl -fsSL "$url" -o "$tmp_file"
    elif command_exists wget; then
        wget -q "$url" -O "$tmp_file"
    fi

    if [[ ! -s "$tmp_file" ]]; then
        error "Failed to download binary from: $url"
        rm -f "$tmp_file"
        exit 1
    fi

    echo "$tmp_file"
}

install_binary() {
    local tmp_file="$1"
    local install_path="${INSTALL_DIR}/${BINARY_NAME}"

    info "Installing to ${install_path}..."

    chmod +x "$tmp_file"

    if [[ -w "$INSTALL_DIR" ]]; then
        mv "$tmp_file" "$install_path"
    else
        warn "Elevated permissions required to install to ${INSTALL_DIR}"
        sudo mv "$tmp_file" "$install_path"
    fi

    success "Binary installed successfully!"
}

# =============================================================================
# Configuration Prompts
# =============================================================================

prompt() {
    local prompt_text="$1"
    local default="$2"
    local result

    if [[ -n "$default" ]]; then
        read -rp "$(echo -e "${CYAN}?${NC} ${prompt_text} [${default}]: ")" result
        echo "${result:-$default}"
    else
        read -rp "$(echo -e "${CYAN}?${NC} ${prompt_text}: ")" result
        echo "$result"
    fi
}

prompt_secret() {
    local prompt_text="$1"
    local result

    read -rsp "$(echo -e "${CYAN}?${NC} ${prompt_text}: ")" result
    echo
    echo "$result"
}

prompt_choice() {
    local prompt_text="$1"
    shift
    local options=("$@")
    local choice

    echo -e "${CYAN}?${NC} ${prompt_text}"
    for i in "${!options[@]}"; do
        echo "  $((i + 1))) ${options[$i]}"
    done

    while true; do
        read -rp "  Enter choice [1-${#options[@]}]: " choice
        if [[ "$choice" =~ ^[0-9]+$ ]] && [[ "$choice" -ge 1 ]] && [[ "$choice" -le "${#options[@]}" ]]; then
            echo "${options[$((choice - 1))]}"
            return
        fi
        echo "  Invalid choice. Please enter a number between 1 and ${#options[@]}."
    done
}

prompt_yes_no() {
    local prompt_text="$1"
    local default="${2:-y}"
    local result

    if [[ "$default" == "y" ]]; then
        read -rp "$(echo -e "${CYAN}?${NC} ${prompt_text} [Y/n]: ")" result
        result="${result:-y}"
    else
        read -rp "$(echo -e "${CYAN}?${NC} ${prompt_text} [y/N]: ")" result
        result="${result:-n}"
    fi

    [[ "$result" =~ ^[Yy] ]]
}

# =============================================================================
# Configuration Setup
# =============================================================================

setup_telegram_config() {
    echo
    info "Configuring Telegram..."
    echo "  To get a bot token, message @BotFather on Telegram and use /newbot"
    echo "  To get your chat ID, message @userinfobot on Telegram"
    echo

    TELEGRAM_BOT_TOKEN=$(prompt_secret "Telegram bot token")
    TELEGRAM_CHAT_ID=$(prompt "Telegram chat ID")

    if [[ -z "$TELEGRAM_BOT_TOKEN" ]] || [[ -z "$TELEGRAM_CHAT_ID" ]]; then
        error "Telegram bot token and chat ID are required"
        exit 1
    fi
}

setup_discord_config() {
    echo
    info "Configuring Discord..."
    echo "  1. Go to https://discord.com/developers/applications"
    echo "  2. Create a new application and get the bot token"
    echo "  3. Enable 'Message Content Intent' in Bot settings"
    echo "  4. Invite the bot with 'Send Messages' permission"
    echo "  5. Get your user ID (enable Developer Mode, right-click your name)"
    echo

    DISCORD_BOT_TOKEN=$(prompt_secret "Discord bot token")
    DISCORD_USER_ID=$(prompt "Discord user ID")

    if [[ -z "$DISCORD_BOT_TOKEN" ]] || [[ -z "$DISCORD_USER_ID" ]]; then
        error "Discord bot token and user ID are required"
        exit 1
    fi
}

setup_signal_config() {
    echo
    info "Configuring Signal..."
    echo "  Note: Signal requires linking as a secondary device after installation."
    echo "  You'll need to run: ${BINARY_NAME} signal-link --device-name \"claude-code\""
    echo

    SIGNAL_PHONE=$(prompt "Your Signal phone number (e.g., +1234567890)")
    SIGNAL_DEVICE_NAME=$(prompt "Device name" "claude-code")

    if [[ -z "$SIGNAL_PHONE" ]]; then
        error "Signal phone number is required"
        exit 1
    fi
}

create_config_file() {
    local primary="$1"

    mkdir -p "$CLAUDE_DIR"

    info "Creating configuration file at ${CONFIG_FILE}..."

    # Build JSON config
    local config='{'
    config+='"messengers": {'

    # Telegram config (always included as fallback)
    if [[ -n "$TELEGRAM_BOT_TOKEN" ]]; then
        config+='"telegram": {'
        config+='"enabled": true,'
        config+="\"bot_token\": \"${TELEGRAM_BOT_TOKEN}\","
        config+="\"chat_id\": \"${TELEGRAM_CHAT_ID}\""
        config+='}'
    fi

    # Discord config
    if [[ -n "$DISCORD_BOT_TOKEN" ]]; then
        [[ -n "$TELEGRAM_BOT_TOKEN" ]] && config+=','
        config+='"discord": {'
        config+='"enabled": true,'
        config+="\"bot_token\": \"${DISCORD_BOT_TOKEN}\","
        config+="\"user_id\": \"${DISCORD_USER_ID}\""
        config+='}'
    fi

    # Signal config
    if [[ -n "$SIGNAL_PHONE" ]]; then
        [[ -n "$TELEGRAM_BOT_TOKEN" ]] || [[ -n "$DISCORD_BOT_TOKEN" ]] && config+=','
        config+='"signal": {'
        config+='"enabled": true,'
        config+="\"phone_number\": \"${SIGNAL_PHONE}\","
        config+="\"device_name\": \"${SIGNAL_DEVICE_NAME}\","
        config+="\"data_path\": \"${CLAUDE_DIR}/signal_data\""
        config+='}'
    fi

    config+='},'
    config+='"preferences": {'
    config+="\"primary_messenger\": \"${primary}\","
    config+='"timeout_seconds": 300'
    config+='}'
    config+='}'

    # Pretty print with python or jq if available
    if command_exists python3; then
        echo "$config" | python3 -m json.tool > "$CONFIG_FILE"
    elif command_exists jq; then
        echo "$config" | jq '.' > "$CONFIG_FILE"
    else
        echo "$config" > "$CONFIG_FILE"
    fi

    chmod 600 "$CONFIG_FILE"
    success "Configuration saved!"
}

# =============================================================================
# Claude Code Hooks Setup
# =============================================================================

setup_claude_hooks() {
    info "Setting up Claude Code hooks..."

    local hooks_config
    hooks_config=$(cat <<'HOOKS_JSON'
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
HOOKS_JSON
)

    if [[ -f "$SETTINGS_FILE" ]]; then
        warn "Existing settings.json found at ${SETTINGS_FILE}"
        echo
        echo "Add the following hooks configuration manually:"
        echo
        echo "$hooks_config"
        echo
    else
        mkdir -p "$CLAUDE_DIR"
        echo "$hooks_config" > "$SETTINGS_FILE"
        success "Claude Code hooks configured!"
    fi
}

# =============================================================================
# Main Installation Flow
# =============================================================================

show_help() {
    cat <<EOF
Claude Code Remote - Installation Script

Usage:
  install.sh [options]

Options:
  --help, -h          Show this help message
  --skip-config       Skip configuration setup
  --skip-hooks        Skip Claude Code hooks setup
  --install-dir DIR   Install binary to DIR (default: /usr/local/bin)
  --version VER       Install specific version (default: latest)

Examples:
  # Interactive installation
  curl -fsSL https://raw.githubusercontent.com/kyujin-cho/claude-code-remote/main/install.sh | bash

  # Skip configuration (binary only)
  curl -fsSL ... | bash -s -- --skip-config

  # Install to custom directory
  curl -fsSL ... | bash -s -- --install-dir ~/.local/bin
EOF
}

main() {
    local skip_config=false
    local skip_hooks=false
    local version=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                show_help
                exit 0
                ;;
            --skip-config)
                skip_config=true
                shift
                ;;
            --skip-hooks)
                skip_hooks=true
                shift
                ;;
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --version)
                version="$2"
                shift 2
                ;;
            *)
                error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    echo
    echo -e "${BOLD}Claude Code Remote Installer${NC}"
    echo "=============================="
    echo

    # Check for required tools
    if ! command_exists curl && ! command_exists wget; then
        error "Either curl or wget is required"
        exit 1
    fi

    # Detect platform
    detect_platform
    info "Detected platform: ${PLATFORM}"

    # Get version
    if [[ -z "$version" ]]; then
        info "Fetching latest version..."
        version=$(get_latest_version)
        if [[ -z "$version" ]]; then
            error "Failed to determine latest version"
            exit 1
        fi
    fi
    info "Version: ${version}"

    # Download and install binary
    local tmp_file
    tmp_file=$(download_binary "$version")
    install_binary "$tmp_file"

    # Verify installation
    if command_exists "$BINARY_NAME"; then
        success "Installation verified: $(command -v "$BINARY_NAME")"
    else
        warn "Binary installed but not in PATH. Add ${INSTALL_DIR} to your PATH."
    fi

    # Configuration setup
    if [[ "$skip_config" == false ]]; then
        echo
        info "Configuration Setup"
        echo

        # Check for existing config
        if [[ -f "$CONFIG_FILE" ]] || [[ -f "$LEGACY_CONFIG_FILE" ]]; then
            if ! prompt_yes_no "Existing configuration found. Overwrite?"; then
                skip_config=true
                info "Keeping existing configuration."
            fi
        fi

        if [[ "$skip_config" == false ]]; then
            # Choose primary messenger
            echo
            PRIMARY_MESSENGER=$(prompt_choice "Select primary messenger:" "telegram" "discord" "signal")

            case "$PRIMARY_MESSENGER" in
                telegram)
                    setup_telegram_config
                    ;;
                discord)
                    setup_discord_config
                    # Also ask for Telegram as fallback
                    if prompt_yes_no "Also configure Telegram as fallback?" "y"; then
                        setup_telegram_config
                    fi
                    ;;
                signal)
                    setup_signal_config
                    # Also ask for Telegram as fallback
                    if prompt_yes_no "Also configure Telegram as fallback?" "y"; then
                        setup_telegram_config
                    fi
                    ;;
            esac

            create_config_file "$PRIMARY_MESSENGER"
        fi
    fi

    # Claude Code hooks setup
    if [[ "$skip_hooks" == false ]]; then
        echo
        if prompt_yes_no "Configure Claude Code hooks?" "y"; then
            setup_claude_hooks
        fi
    fi

    # Final message
    echo
    success "Installation complete!"
    echo
    echo "Next steps:"
    echo "  1. Verify installation: ${BINARY_NAME} status"
    echo "  2. Test the hook: echo '{\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"ls\"}}' | ${BINARY_NAME} hook"
    if [[ "$PRIMARY_MESSENGER" == "signal" ]]; then
        echo "  3. Link Signal device: ${BINARY_NAME} signal-link --device-name \"${SIGNAL_DEVICE_NAME:-claude-code}\""
    fi
    echo
    echo "Documentation: https://github.com/${REPO}"
    echo
}

main "$@"
