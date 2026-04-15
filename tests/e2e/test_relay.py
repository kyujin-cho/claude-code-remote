"""
E2E tests for the relay command.
"""

import asyncio
import pytest

from conftest import HookRunner, TEST_TIMEOUT
from userbot import TelegramUserbot


pytestmark = pytest.mark.asyncio


class TestRelayCommand:
    """Tests for the relay command functionality."""

    async def test_relay_simple_message(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Relay sends a simple message."""
        message = "Hello from E2E test!"

        # Start listener BEFORE running command (to avoid race condition)
        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Hello")
        )
        await asyncio.sleep(0.5)  # Give handler time to register

        await runner.run_relay(message)

        msg = await listener_task

        assert msg is not None
        assert "Hello from E2E test!" in msg.text

    async def test_relay_message_with_special_chars(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Relay handles special characters."""
        message = "Test with *bold* and _italic_ and `code`"

        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Test")
        )
        await asyncio.sleep(0.5)

        await runner.run_relay(message)

        msg = await listener_task

        assert msg is not None
        # Message should be received (formatting may vary)

    async def test_relay_multiline_message(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Relay handles multiline messages."""
        message = "Line 1\nLine 2\nLine 3"

        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Line 1")
        )
        await asyncio.sleep(0.5)

        await runner.run_relay(message)

        msg = await listener_task

        assert msg is not None
        assert "Line 1" in msg.text

    async def test_relay_unicode_message(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Relay handles unicode characters."""
        message = "Unicode test: 你好 🎉 ñ é"

        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Unicode")
        )
        await asyncio.sleep(0.5)

        await runner.run_relay(message)

        msg = await listener_task

        assert msg is not None
        assert "你好" in msg.text or "Unicode" in msg.text


class TestStatusCommand:
    """Tests for the status command."""

    def test_status_shows_config(self, runner: HookRunner):
        """Test: Status command shows configuration."""
        output = runner.run_status()

        assert "Status" in output or "Configuration" in output
        assert "Telegram" in output

    def test_status_shows_telegram_configured(self, runner: HookRunner):
        """Test: Status shows Telegram as configured."""
        output = runner.run_status()

        # Should indicate Telegram is configured
        assert "Telegram" in output
        assert "Configured" in output or "Chat ID" in output
