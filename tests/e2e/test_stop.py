"""
E2E tests for the stop handler (job completion notifications).
"""

import asyncio
import pytest

from conftest import HookRunner, TEST_TIMEOUT
from userbot import TelegramUserbot


pytestmark = pytest.mark.asyncio


class TestStopNotification:
    """Tests for stop/job completion notifications."""

    async def test_stop_sends_notification(
        self,
        userbot: TelegramUserbot,
        runner: HookRunner,
        sample_stop_input: dict,
    ):
        """Test: Stop handler sends job completion notification."""
        # Start listener BEFORE running command
        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Job Completed")
        )
        await asyncio.sleep(0.5)

        await runner.run_stop(sample_stop_input)

        msg = await listener_task

        assert msg is not None
        assert "Job Completed" in msg.text
        assert "test-project" in msg.text

    async def test_stop_includes_summary(
        self,
        userbot: TelegramUserbot,
        runner: HookRunner,
        sample_stop_input: dict,
    ):
        """Test: Stop notification includes summary from transcript."""
        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Job Completed")
        )
        await asyncio.sleep(0.5)

        await runner.run_stop(sample_stop_input)

        msg = await listener_task

        assert msg is not None
        # Should include last assistant message from transcript
        assert "completed successfully" in msg.text.lower() or "Summary" in msg.text

    async def test_stop_skips_when_hook_active(
        self, userbot: TelegramUserbot, runner: HookRunner, sample_stop_input: dict
    ):
        """Test: Stop handler skips notification when stop_hook_active is true."""
        # Modify input to set stop_hook_active
        sample_stop_input["stop_hook_active"] = True

        # This should not raise and should not send a message
        await runner.run_stop(sample_stop_input)

        # Try to get a message - should timeout (no message sent)
        try:
            listener_task = asyncio.create_task(
                userbot.wait_for_message(timeout=3, contains="Job Completed")
            )
            msg = await listener_task
            # If we get here, a message was unexpectedly sent
            pytest.fail("Stop notification was sent when stop_hook_active was True")
        except TimeoutError:
            # Expected - no message should be sent
            pass

    async def test_stop_handles_missing_transcript(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Stop handler works even without transcript file."""
        input_data = {
            "session_id": "test-123",
            "transcript_path": "/nonexistent/path.jsonl",
            "cwd": "/home/user/my-project",
            "stop_hook_active": False,
        }

        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=TEST_TIMEOUT, contains="Job Completed")
        )
        await asyncio.sleep(0.5)

        await runner.run_stop(input_data)

        msg = await listener_task

        assert msg is not None
        assert "my-project" in msg.text
