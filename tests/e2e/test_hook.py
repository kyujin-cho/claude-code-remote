"""
E2E tests for the hook handler (permission requests).
"""

import asyncio
import json
import os

import pytest

from conftest import HookRunner, TEST_TIMEOUT
from userbot import TelegramUserbot


pytestmark = pytest.mark.asyncio


class TestHookAllow:
    """Tests for Allow button functionality."""

    async def test_hook_allow_bash(
        self, userbot: TelegramUserbot, runner: HookRunner, sample_bash_input: dict
    ):
        """Test: Send Bash permission request -> Click Allow -> Returns allow."""
        # Start hook in background
        proc = await runner.run_hook_async(sample_bash_input)

        try:
            # Wait for message from bot
            msg = await userbot.wait_for_message(
                timeout=TEST_TIMEOUT, contains="Permission Request"
            )

            assert msg is not None
            assert "Bash" in msg.text
            assert "ls -la" in msg.text

            # Click Allow button
            await userbot.click_button(msg, "Allow")

            # Wait for hook to complete and get response
            stdout, stderr = await asyncio.wait_for(
                proc.communicate(), timeout=TEST_TIMEOUT
            )

            assert proc.returncode == 0, f"Hook failed: {stderr.decode()}"

            response = json.loads(stdout.decode())
            assert response["hookSpecificOutput"]["decision"]["behavior"] == "allow"

        except Exception:
            proc.kill()
            raise

    async def test_hook_allow_edit(
        self, userbot: TelegramUserbot, runner: HookRunner, sample_edit_input: dict
    ):
        """Test: Send Edit permission request -> Click Allow -> Returns allow."""
        proc = await runner.run_hook_async(sample_edit_input)

        try:
            msg = await userbot.wait_for_message(
                timeout=TEST_TIMEOUT, contains="Permission Request"
            )

            assert "Edit" in msg.text
            assert "test.txt" in msg.text

            await userbot.click_button(msg, "Allow")

            stdout, stderr = await asyncio.wait_for(
                proc.communicate(), timeout=TEST_TIMEOUT
            )

            response = json.loads(stdout.decode())
            assert response["hookSpecificOutput"]["decision"]["behavior"] == "allow"

        except Exception:
            proc.kill()
            raise


class TestHookDeny:
    """Tests for Deny button functionality."""

    async def test_hook_deny_bash(
        self, userbot: TelegramUserbot, runner: HookRunner, sample_bash_input: dict
    ):
        """Test: Send permission request -> Click Deny -> Returns deny."""
        proc = await runner.run_hook_async(sample_bash_input)

        try:
            msg = await userbot.wait_for_message(
                timeout=TEST_TIMEOUT, contains="Permission Request"
            )

            assert msg is not None

            # Click Deny button
            await userbot.click_button(msg, "Deny")

            stdout, stderr = await asyncio.wait_for(
                proc.communicate(), timeout=TEST_TIMEOUT
            )

            response = json.loads(stdout.decode())
            assert response["hookSpecificOutput"]["decision"]["behavior"] == "deny"

        except Exception:
            proc.kill()
            raise


class TestHookAlwaysAllow:
    """Tests for Always Allow button functionality."""

    async def test_hook_always_allow(
        self,
        userbot: TelegramUserbot,
        runner: HookRunner,
        sample_bash_input: dict,
        always_allow_path,
    ):
        """Test: Click Always Allow -> Returns allow + saves to always_allow.json."""
        proc = await runner.run_hook_async(sample_bash_input)

        try:
            msg = await userbot.wait_for_message(
                timeout=TEST_TIMEOUT, contains="Permission Request"
            )

            assert msg is not None

            # Click Always Allow button
            await userbot.click_button(msg, "Always")

            stdout, stderr = await asyncio.wait_for(
                proc.communicate(), timeout=TEST_TIMEOUT
            )

            response = json.loads(stdout.decode())
            assert response["hookSpecificOutput"]["decision"]["behavior"] == "allow"

            # Verify tool was saved to always_allow.json
            assert always_allow_path.exists()
            always_allow = json.loads(always_allow_path.read_text())
            assert "Bash" in always_allow.get("tools", [])

        except Exception:
            proc.kill()
            raise

    async def test_hook_auto_approve_after_always_allow(
        self,
        userbot: TelegramUserbot,
        runner: HookRunner,
        sample_bash_input: dict,
        always_allow_path,
    ):
        """Test: After Always Allow, subsequent requests are auto-approved."""
        # Pre-populate always_allow.json
        always_allow_path.write_text(json.dumps({"tools": ["Bash"]}))

        # Start listener BEFORE running hook (to catch auto-approval notification)
        listener_task = asyncio.create_task(
            userbot.wait_for_message(timeout=10, contains="Auto-Approved")
        )
        await asyncio.sleep(0.5)

        # Run hook - should auto-approve without button click
        response = await runner.run_hook(sample_bash_input, timeout=TEST_TIMEOUT)

        assert response["hookSpecificOutput"]["decision"]["behavior"] == "allow"

        # Should still receive notification about auto-approval
        msg = await listener_task
        assert msg is not None
        assert "Bash" in msg.text


class TestHookTimeout:
    """Tests for timeout behavior."""

    @pytest.mark.slow
    async def test_hook_timeout_denies(
        self, userbot: TelegramUserbot, runner: HookRunner, hook_config
    ):
        """Test: No response within timeout -> Returns deny."""
        # Use a very short timeout for testing
        config = json.loads(hook_config.read_text())
        config["preferences"]["timeout_seconds"] = 3
        hook_config.write_text(json.dumps(config))

        input_data = {"tool_name": "Bash", "tool_input": {"command": "echo test"}}

        proc = await runner.run_hook_async(input_data)

        try:
            # Wait for message but don't click anything
            msg = await userbot.wait_for_message(
                timeout=10, contains="Permission Request"
            )
            assert msg is not None

            # Don't click any button - wait for timeout
            stdout, stderr = await asyncio.wait_for(
                proc.communicate(), timeout=10
            )

            response = json.loads(stdout.decode())
            assert response["hookSpecificOutput"]["decision"]["behavior"] == "deny"

        except Exception:
            proc.kill()
            raise


class TestHookMessageFormat:
    """Tests for message formatting."""

    async def test_bash_command_in_message(
        self, userbot: TelegramUserbot, runner: HookRunner
    ):
        """Test: Bash command is properly displayed in message."""
        input_data = {
            "tool_name": "Bash",
            "tool_input": {"command": "rm -rf /important/data"},
        }

        proc = await runner.run_hook_async(input_data)

        try:
            msg = await userbot.wait_for_message(
                timeout=TEST_TIMEOUT, contains="Permission Request"
            )

            assert "Bash" in msg.text
            assert "rm -rf" in msg.text
            assert "/important/data" in msg.text

            # Clean up - deny the request
            await userbot.click_button(msg, "Deny")
            await proc.communicate()

        except Exception:
            proc.kill()
            raise

    async def test_write_content_in_message(
        self, userbot: TelegramUserbot, runner: HookRunner, sample_write_input: dict
    ):
        """Test: Write content is properly displayed in message."""
        proc = await runner.run_hook_async(sample_write_input)

        try:
            msg = await userbot.wait_for_message(
                timeout=TEST_TIMEOUT, contains="Permission Request"
            )

            assert "Write" in msg.text
            assert "newfile.txt" in msg.text

            # Clean up
            await userbot.click_button(msg, "Deny")
            await proc.communicate()

        except Exception:
            proc.kill()
            raise
