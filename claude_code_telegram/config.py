"""Configuration management for the Telegram bot."""

import json
import os
import socket
from dataclasses import dataclass
from pathlib import Path

from dotenv import load_dotenv


DEFAULT_CONFIG_PATH = Path.home() / ".claude" / "telegram_hook.json"


@dataclass
class Config:
    """Application configuration."""

    telegram_bot_token: str
    telegram_chat_id: str
    hostname: str

    @classmethod
    def from_json(cls, config_path: Path | None = None) -> "Config":
        """Load configuration from JSON file.

        JSON file format:
        {
            "telegram_bot_token": "your_bot_token",
            "telegram_chat_id": "your_chat_id"
        }
        """
        path = config_path or DEFAULT_CONFIG_PATH

        if not path.exists():
            raise FileNotFoundError(f"Configuration file not found: {path}")

        with open(path) as f:
            data = json.load(f)

        token = data.get("telegram_bot_token")
        chat_id = data.get("telegram_chat_id")

        if not token:
            raise ValueError(f"telegram_bot_token is required in {path}")
        if not chat_id:
            raise ValueError(f"telegram_chat_id is required in {path}")

        # Ensure chat_id is a string
        chat_id = str(chat_id)

        return cls(
            telegram_bot_token=token,
            telegram_chat_id=chat_id,
            hostname=socket.gethostname(),
        )

    @classmethod
    def from_env(cls) -> "Config":
        """Load configuration from environment variables."""
        load_dotenv()

        token = os.getenv("TELEGRAM_BOT_TOKEN")
        chat_id = os.getenv("TELEGRAM_CHAT_ID")

        if not token:
            raise ValueError("TELEGRAM_BOT_TOKEN environment variable is required")
        if not chat_id:
            raise ValueError("TELEGRAM_CHAT_ID environment variable is required")

        return cls(
            telegram_bot_token=token,
            telegram_chat_id=chat_id,
            hostname=socket.gethostname(),
        )

    @classmethod
    def load(cls, config_path: Path | None = None) -> "Config":
        """Load configuration from JSON file, falling back to environment variables.

        Priority:
        1. JSON file at specified path
        2. JSON file at default path (~/.claude/telegram_hook.json)
        3. Environment variables
        """
        # Try JSON file first
        path = config_path or DEFAULT_CONFIG_PATH
        if path.exists():
            return cls.from_json(path)

        # Fall back to environment variables
        return cls.from_env()


def get_project_root() -> Path:
    """Get the project root directory."""
    return Path(__file__).parent.parent
