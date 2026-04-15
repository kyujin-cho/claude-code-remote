"""
Pytest configuration and fixtures for E2E tests.
"""

import asyncio
import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import AsyncGenerator, Optional

import pytest
import pytest_asyncio

from userbot import TelegramUserbot


# Get binary path from environment or use default
BINARY_PATH = os.environ.get(
    "CLAUDE_CODE_TELEGRAM_BIN", "/app/target/release/claude-code-telegram"
)

# Bot credentials
BOT_TOKEN = os.environ.get("TELEGRAM_BOT_TOKEN", "")
CHAT_ID = os.environ.get("TELEGRAM_CHAT_ID", "")

# Extract bot ID from token (format: BOT_ID:HASH)
BOT_ID = int(BOT_TOKEN.split(":")[0]) if BOT_TOKEN else 0

# Test timeout
TEST_TIMEOUT = int(os.environ.get("TEST_TIMEOUT", "30"))


@pytest.fixture(scope="session")
def event_loop():
    """Create event loop for async tests."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()


@pytest_asyncio.fixture(scope="session")
async def userbot() -> AsyncGenerator[TelegramUserbot, None]:
    """
    Create and start the Telegram userbot.

    This fixture is session-scoped so we only login once.
    """
    bot = TelegramUserbot.from_env()
    await bot.start()
    bot.set_bot_id(BOT_ID)

    yield bot

    await bot.disconnect()


@pytest.fixture
def config_dir(tmp_path) -> Path:
    """Create a temporary config directory."""
    claude_dir = tmp_path / ".claude"
    claude_dir.mkdir()
    return claude_dir


@pytest.fixture
def hook_config(config_dir) -> Path:
    """Create hook config file with bot credentials."""
    config = {
        "messengers": {
            "telegram": {
                "enabled": True,
                "bot_token": BOT_TOKEN,
                "chat_id": CHAT_ID,
            }
        },
        "preferences": {"primary_messenger": "telegram", "timeout_seconds": TEST_TIMEOUT},
    }

    config_path = config_dir / "hook_config.json"
    config_path.write_text(json.dumps(config, indent=2))

    return config_path


@pytest.fixture
def always_allow_path(config_dir) -> Path:
    """Path to always_allow.json file."""
    return config_dir / "always_allow.json"


class HookRunner:
    """Helper class to run claude-code-telegram commands."""

    def __init__(self, config_dir: Path):
        self.config_dir = config_dir
        self.env = os.environ.copy()
        self.env["HOME"] = str(config_dir.parent)

    async def run_hook(self, input_data: dict, timeout: float = 60.0) -> dict:
        """
        Run the hook command with JSON input.

        Args:
            input_data: Dict to send as JSON via stdin
            timeout: Maximum time to wait for response

        Returns:
            Parsed JSON response
        """
        proc = await asyncio.create_subprocess_exec(
            BINARY_PATH,
            "hook",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=self.env,
        )

        input_json = json.dumps(input_data).encode()

        try:
            stdout, stderr = await asyncio.wait_for(
                proc.communicate(input=input_json), timeout=timeout
            )
        except asyncio.TimeoutError:
            proc.kill()
            await proc.wait()
            raise

        if proc.returncode != 0:
            raise RuntimeError(f"Hook failed: {stderr.decode()}")

        return json.loads(stdout.decode())

    async def run_hook_async(
        self, input_data: dict
    ) -> asyncio.subprocess.Process:
        """
        Start the hook command without waiting for completion.

        Useful for tests that need to interact with the bot
        while the hook is waiting.

        Returns:
            The subprocess.Process object
        """
        proc = await asyncio.create_subprocess_exec(
            BINARY_PATH,
            "hook",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=self.env,
        )

        # Write input but don't close stdin yet
        input_json = json.dumps(input_data).encode()
        proc.stdin.write(input_json)
        await proc.stdin.drain()
        proc.stdin.close()

        return proc

    async def run_stop(self, input_data: dict) -> None:
        """Run the stop command with JSON input."""
        proc = await asyncio.create_subprocess_exec(
            BINARY_PATH,
            "stop",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=self.env,
        )

        input_json = json.dumps(input_data).encode()
        stdout, stderr = await proc.communicate(input=input_json)

        if proc.returncode != 0:
            raise RuntimeError(f"Stop failed: {stderr.decode()}")

    async def run_notify(self, input_data: dict) -> None:
        """Run the notify command with JSON input."""
        proc = await asyncio.create_subprocess_exec(
            BINARY_PATH,
            "notify",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=self.env,
        )

        input_json = json.dumps(input_data).encode()
        stdout, stderr = await proc.communicate(input=input_json)

        if proc.returncode != 0:
            raise RuntimeError(f"Notify failed: {stderr.decode()}")

    async def run_relay(self, message: str) -> None:
        """Run the relay command with a message."""
        proc = await asyncio.create_subprocess_exec(
            BINARY_PATH,
            "relay",
            message,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=self.env,
        )

        stdout, stderr = await proc.communicate()

        if proc.returncode != 0:
            raise RuntimeError(f"Relay failed: {stderr.decode()}")

    def run_status(self) -> str:
        """Run the status command synchronously."""
        result = subprocess.run(
            [BINARY_PATH, "status"],
            capture_output=True,
            text=True,
            env=self.env,
        )
        return result.stdout


@pytest.fixture
def runner(hook_config) -> HookRunner:
    """Create a HookRunner with proper config."""
    return HookRunner(hook_config.parent)


@pytest.fixture
def sample_bash_input() -> dict:
    """Sample Bash permission request input."""
    return {"tool_name": "Bash", "tool_input": {"command": "ls -la /tmp"}}


@pytest.fixture
def sample_edit_input() -> dict:
    """Sample Edit permission request input."""
    return {
        "tool_name": "Edit",
        "tool_input": {
            "file_path": "/tmp/test.txt",
            "old_string": "hello",
            "new_string": "world",
        },
    }


@pytest.fixture
def sample_write_input() -> dict:
    """Sample Write permission request input."""
    return {
        "tool_name": "Write",
        "tool_input": {"file_path": "/tmp/newfile.txt", "content": "Hello, World!"},
    }


@pytest.fixture
def sample_stop_input(tmp_path) -> dict:
    """Sample Stop hook input with transcript."""
    # Create a mock transcript file
    transcript_path = tmp_path / "transcript.jsonl"
    transcript_data = [
        {"type": "user", "message": {"content": [{"type": "text", "text": "Hello"}]}},
        {
            "type": "assistant",
            "message": {"content": [{"type": "text", "text": "Task completed successfully!"}]},
        },
    ]
    with open(transcript_path, "w") as f:
        for entry in transcript_data:
            f.write(json.dumps(entry) + "\n")

    return {
        "session_id": "test-session-123",
        "transcript_path": str(transcript_path),
        "cwd": "/home/user/test-project",
        "stop_hook_active": False,
    }


@pytest.fixture
def sample_notification_input() -> dict:
    """Sample notification input."""
    return {
        "notification_type": "idle_prompt",
        "message": "Claude is waiting for input",
        "session_id": "test-session-123",
        "cwd": "/home/user/test-project",
    }
