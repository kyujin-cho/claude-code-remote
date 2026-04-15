"""
E2E tests for the notification handler.
"""

import asyncio
import pytest

from conftest import HookRunner, TEST_TIMEOUT
from userbot import TelegramUserbot


pytestmark = pytest.mark.asyncio


class TestNotificationHandler:
    """Tests for notification relay functionality."""

    async def test_idle_notification(
        self,
        userbot: TelegramUserbot,
        runner: HookRunner,
        sample_notification_input: dict,
    ):
        """Test: Idle prompt notification is sent."""
        # Start listener BEFORE running command
        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Idle")
        )
        await asyncio.sleep(0.5)

        await runner.run_notify(sample_notification_input)

        msg = await listener_task

        assert msg is not None
        assert "Idle" in msg.text or "Waiting" in msg.text
        assert "test-project" in msg.text

    async def test_permission_prompt_notification(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Permission prompt notification is sent."""
        input_data = {
            "notification_type": "permission_prompt",
            "message": "Claude needs permission to run bash",
            "session_id": "test-123",
            "cwd": "/home/user/project",
        }

        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Permission")
        )
        await asyncio.sleep(0.5)

        await runner.run_notify(input_data)

        msg = await listener_task

        assert msg is not None
        assert "Permission" in msg.text

    async def test_custom_notification(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Custom notification type is sent."""
        input_data = {
            "notification_type": "custom",
            "message": "This is a custom notification message",
            "session_id": "test-123",
            "cwd": "/home/user/project",
        }

        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Notification")
        )
        await asyncio.sleep(0.5)

        await runner.run_notify(input_data)

        msg = await listener_task

        assert msg is not None
        assert "custom notification" in msg.text.lower()

    async def test_notification_includes_hostname(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Notification includes hostname."""
        input_data = {
            "notification_type": "idle_prompt",
            "message": "Test message",
            "session_id": "test-123",
            "cwd": "/home/user/project",
        }

        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT)
        )
        await asyncio.sleep(0.5)

        await runner.run_notify(input_data)

        msg = await listener_task

        assert msg is not None
        assert "Host" in msg.text

    async def test_notification_truncates_long_message(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Long messages are truncated."""
        long_message = "A" * 1000  # 1000 character message

        input_data = {
            "notification_type": "custom",
            "message": long_message,
            "session_id": "test-123",
            "cwd": "/home/user/project",
        }

        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT)
        )
        await asyncio.sleep(0.5)

        await runner.run_notify(input_data)

        msg = await listener_task

        assert msg is not None
        # Message should be truncated (500 chars + "...")
        assert len(msg.text) < 1000 or "..." in msg.text
