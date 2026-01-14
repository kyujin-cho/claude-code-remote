"""Tests for configuration module."""

import json
import os
import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest

from claude_code_telegram.config import Config, get_project_root


class TestConfig:
    """Tests for Config class."""

    def test_from_env_success(self) -> None:
        """Test successful config loading from environment."""
        with patch("claude_code_telegram.config.load_dotenv"):
            with patch("claude_code_telegram.config.socket.gethostname", return_value="test-host"):
                with patch.dict(
                    os.environ,
                    {
                        "TELEGRAM_BOT_TOKEN": "test_token",
                        "TELEGRAM_CHAT_ID": "123456",
                    },
                    clear=True,
                ):
                    config = Config.from_env()
                    assert config.telegram_bot_token == "test_token"
                    assert config.telegram_chat_id == "123456"
                    assert config.hostname == "test-host"

    def test_from_env_missing_token(self) -> None:
        """Test error when bot token is missing."""
        with patch("claude_code_telegram.config.load_dotenv"):
            with patch.dict(os.environ, {"TELEGRAM_CHAT_ID": "123456"}, clear=True):
                with pytest.raises(ValueError, match="TELEGRAM_BOT_TOKEN"):
                    Config.from_env()

    def test_from_env_missing_chat_id(self) -> None:
        """Test error when chat ID is missing."""
        with patch("claude_code_telegram.config.load_dotenv"):
            with patch.dict(os.environ, {"TELEGRAM_BOT_TOKEN": "test_token"}, clear=True):
                with pytest.raises(ValueError, match="TELEGRAM_CHAT_ID"):
                    Config.from_env()


class TestConfigFromJson:
    """Tests for Config.from_json method."""

    def test_from_json_success(self) -> None:
        """Test successful config loading from JSON file."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            config_path.write_text(json.dumps({
                "telegram_bot_token": "json_token",
                "telegram_chat_id": "654321"
            }))

            with patch("claude_code_telegram.config.socket.gethostname", return_value="json-host"):
                config = Config.from_json(config_path)
                assert config.telegram_bot_token == "json_token"
                assert config.telegram_chat_id == "654321"
                assert config.hostname == "json-host"

    def test_from_json_chat_id_as_int(self) -> None:
        """Test that chat_id is converted to string if provided as int."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            config_path.write_text(json.dumps({
                "telegram_bot_token": "token",
                "telegram_chat_id": 123456  # int, not string
            }))

            with patch("claude_code_telegram.config.socket.gethostname", return_value="host"):
                config = Config.from_json(config_path)
                assert config.telegram_chat_id == "123456"
                assert isinstance(config.telegram_chat_id, str)

    def test_from_json_file_not_found(self) -> None:
        """Test error when config file doesn't exist."""
        with pytest.raises(FileNotFoundError):
            Config.from_json(Path("/nonexistent/path/config.json"))

    def test_from_json_missing_token(self) -> None:
        """Test error when token is missing in JSON."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            config_path.write_text(json.dumps({
                "telegram_chat_id": "123456"
            }))

            with pytest.raises(ValueError, match="telegram_bot_token"):
                Config.from_json(config_path)

    def test_from_json_missing_chat_id(self) -> None:
        """Test error when chat_id is missing in JSON."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            config_path.write_text(json.dumps({
                "telegram_bot_token": "token"
            }))

            with pytest.raises(ValueError, match="telegram_chat_id"):
                Config.from_json(config_path)


class TestConfigLoad:
    """Tests for Config.load method."""

    def test_load_prefers_json(self) -> None:
        """Test that load() prefers JSON file over env vars."""
        with tempfile.TemporaryDirectory() as tmpdir:
            config_path = Path(tmpdir) / "config.json"
            config_path.write_text(json.dumps({
                "telegram_bot_token": "json_token",
                "telegram_chat_id": "111"
            }))

            with patch("claude_code_telegram.config.socket.gethostname", return_value="host"):
                with patch.dict(os.environ, {
                    "TELEGRAM_BOT_TOKEN": "env_token",
                    "TELEGRAM_CHAT_ID": "222"
                }):
                    config = Config.load(config_path)
                    assert config.telegram_bot_token == "json_token"
                    assert config.telegram_chat_id == "111"

    def test_load_falls_back_to_env(self) -> None:
        """Test that load() falls back to env vars when JSON doesn't exist."""
        with patch("claude_code_telegram.config.load_dotenv"):
            with patch("claude_code_telegram.config.socket.gethostname", return_value="host"):
                with patch.dict(os.environ, {
                    "TELEGRAM_BOT_TOKEN": "env_token",
                    "TELEGRAM_CHAT_ID": "222"
                }, clear=True):
                    # Use a path that doesn't exist
                    config = Config.load(Path("/nonexistent/config.json"))
                    assert config.telegram_bot_token == "env_token"
                    assert config.telegram_chat_id == "222"


class TestGetProjectRoot:
    """Tests for get_project_root function."""

    def test_returns_path(self) -> None:
        """Test that project root is returned as a Path."""
        root = get_project_root()
        assert root.exists()
        assert (root / "claude_code_telegram").exists()
