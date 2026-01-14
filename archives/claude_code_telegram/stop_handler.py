#!/usr/bin/env python3
"""Claude Code stop hook handler for job completion notifications.

This script is called by Claude Code hooks when the agent finishes responding.
It sends a notification to Telegram to alert the user.
"""

import asyncio
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from telegram import Bot

from claude_code_telegram.config import Config


@dataclass
class StopEvent:
    """Represents a Claude Code stop event."""

    session_id: str
    transcript_path: str
    cwd: str
    stop_hook_active: bool

    @classmethod
    def from_hook_input(cls, data: dict[str, Any]) -> "StopEvent":
        """Parse stop event from Claude Code hook input."""
        return cls(
            session_id=data.get("session_id", ""),
            transcript_path=data.get("transcript_path", ""),
            cwd=data.get("cwd", ""),
            stop_hook_active=data.get("stop_hook_active", False),
        )

    def get_last_assistant_message(self) -> str | None:
        """Extract the last assistant message from the transcript."""
        if not self.transcript_path:
            return None

        transcript_path = Path(self.transcript_path)
        if not transcript_path.exists():
            return None

        try:
            last_assistant_msg = None
            with open(transcript_path) as f:
                for line in f:
                    try:
                        entry = json.loads(line)
                        if entry.get("type") == "assistant":
                            message = entry.get("message", {})
                            content = message.get("content", [])
                            # Extract text from content blocks
                            for block in content:
                                if isinstance(block, dict) and block.get("type") == "text":
                                    last_assistant_msg = block.get("text", "")
                    except json.JSONDecodeError:
                        continue
            return last_assistant_msg
        except Exception:
            return None


class StopNotifier:
    """Sends stop notifications via Telegram."""

    def __init__(self, config: Config) -> None:
        self.config = config

    async def send_notification(self, event: StopEvent) -> None:
        """Send job completion notification to Telegram."""
        # Skip if this is a continuation from a stop hook to prevent loops
        if event.stop_hook_active:
            return

        bot = Bot(token=self.config.telegram_bot_token)

        # Get project name from cwd
        project_name = Path(event.cwd).name if event.cwd else "Unknown"

        # Try to get last assistant message for context
        last_message = event.get_last_assistant_message()
        summary = ""
        if last_message:
            # Truncate to reasonable length
            truncated = last_message[:300]
            if len(last_message) > 300:
                truncated += "..."
            summary = f"\n\n*Summary:*\n{truncated}"

        lines = [
            "✅ *Job Completed*",
            f"🖥️ *Host:* `{self.config.hostname}`",
            f"📁 *Project:* `{project_name}`",
            summary,
        ]

        async with bot:
            await bot.send_message(
                chat_id=self.config.telegram_chat_id,
                text="\n".join(lines),
                parse_mode="Markdown",
            )


async def async_main() -> None:
    """Async main entry point for the stop handler."""
    input_data = json.loads(sys.stdin.read())

    config = Config.load()
    event = StopEvent.from_hook_input(input_data)
    notifier = StopNotifier(config)

    await notifier.send_notification(event)


def main() -> None:
    """Main entry point for the stop handler."""
    asyncio.run(async_main())


if __name__ == "__main__":
    main()
