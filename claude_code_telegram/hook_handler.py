#!/usr/bin/env python3
"""Claude Code hook handler for permission requests.

This script is called by Claude Code hooks and handles permission requests
by sending them to a Telegram bot and waiting for user decisions.
"""

import asyncio
import json
import sys
import uuid
from dataclasses import dataclass
from typing import Any

from telegram import InlineKeyboardButton, InlineKeyboardMarkup
from telegram.ext import Application, CallbackQueryHandler, ContextTypes

from claude_code_telegram.always_allow import AlwaysAllowManager
from claude_code_telegram.config import Config


@dataclass
class PermissionRequest:
    """Represents a Claude Code permission request."""

    tool_name: str
    tool_input: dict[str, Any]
    request_id: str

    @classmethod
    def from_hook_input(cls, data: dict[str, Any]) -> "PermissionRequest":
        """Parse permission request from Claude Code hook input."""
        tool_name = data.get("tool_name", "unknown")
        tool_input = data.get("tool_input", {})
        request_id = str(uuid.uuid4())[:8]
        return cls(tool_name=tool_name, tool_input=tool_input, request_id=request_id)

    def format_message(self, hostname: str | None = None) -> str:
        """Format the permission request as a Telegram message."""
        lines = [
            f"🔐 *Permission Request* `[{self.request_id}]`",
        ]

        if hostname:
            lines.append(f"🖥️ *Host:* `{hostname}`")

        lines.extend([
            "",
            f"*Tool:* `{self.tool_name}`",
        ])

        if self.tool_name == "Bash":
            command = self.tool_input.get("command", "")
            lines.append(f"*Command:*\n```\n{command}\n```")
        elif self.tool_name in ("Edit", "Write"):
            file_path = self.tool_input.get("file_path", "")
            lines.append(f"*File:* `{file_path}`")
            if self.tool_name == "Edit":
                old_string = self.tool_input.get("old_string", "")[:200]
                new_string = self.tool_input.get("new_string", "")[:200]
                lines.append(f"*Old:*\n```\n{old_string}\n```")
                lines.append(f"*New:*\n```\n{new_string}\n```")
        else:
            input_str = json.dumps(self.tool_input, indent=2)[:500]
            lines.append(f"*Input:*\n```json\n{input_str}\n```")

        return "\n".join(lines)


class DecisionHandler:
    """Handles permission decisions via Telegram."""

    def __init__(
        self, config: Config, always_allow_manager: AlwaysAllowManager | None = None
    ) -> None:
        self.config = config
        self.always_allow_manager = always_allow_manager or AlwaysAllowManager()
        self.decision: str | None = None
        self.decision_event = asyncio.Event()

    async def send_auto_approved_notification(self, request: PermissionRequest) -> None:
        """Send a notification for auto-approved tool (no buttons)."""
        app = Application.builder().token(self.config.telegram_bot_token).build()

        lines = [
            f"⚙️ *Auto-Approved* `[{request.request_id}]`",
            f"🖥️ *Host:* `{self.config.hostname}`",
            "",
            f"*Tool:* `{request.tool_name}` _(in always-allow list)_",
        ]

        if request.tool_name == "Bash":
            command = request.tool_input.get("command", "")
            lines.append(f"*Command:*\n```\n{command}\n```")
        elif request.tool_name in ("Edit", "Write"):
            file_path = request.tool_input.get("file_path", "")
            lines.append(f"*File:* `{file_path}`")
        else:
            input_str = json.dumps(request.tool_input, indent=2)[:500]
            lines.append(f"*Input:*\n```json\n{input_str}\n```")

        async with app:
            await app.bot.send_message(
                chat_id=self.config.telegram_chat_id,
                text="\n".join(lines),
                parse_mode="Markdown",
            )

    async def send_request_and_wait(self, request: PermissionRequest) -> str:
        """Send permission request to Telegram and wait for decision."""
        # Check if tool is in always-allow list
        if self.always_allow_manager.is_allowed(request.tool_name):
            await self.send_auto_approved_notification(request)
            return "allow"

        app = Application.builder().token(self.config.telegram_bot_token).build()

        async def handle_callback(
            update: Any, context: ContextTypes.DEFAULT_TYPE
        ) -> None:
            query = update.callback_query
            await query.answer()

            callback_data = query.data
            if callback_data.startswith(request.request_id):
                parts = callback_data.split(":")
                decision = parts[1]

                # Handle "always_allow" decision
                if decision == "always_allow" and len(parts) >= 3:
                    tool_name = parts[2]
                    self.always_allow_manager.add_tool(tool_name)
                    self.decision = "allow"
                    status = f"🔓 Always Allowed (`{tool_name}` added to list)"
                elif decision == "allow":
                    self.decision = "allow"
                    status = "✅ Approved"
                else:
                    self.decision = "deny"
                    status = "❌ Denied"

                await query.edit_message_text(
                    text=f"{request.format_message(self.config.hostname)}\n\n*Status:* {status}",
                    parse_mode="Markdown",
                )
                self.decision_event.set()

        app.add_handler(CallbackQueryHandler(handle_callback))

        keyboard = [
            [
                InlineKeyboardButton(
                    "✅ Allow", callback_data=f"{request.request_id}:allow"
                ),
                InlineKeyboardButton(
                    "❌ Deny", callback_data=f"{request.request_id}:deny"
                ),
            ],
            [
                InlineKeyboardButton(
                    "🔓 Always Allow",
                    callback_data=f"{request.request_id}:always_allow:{request.tool_name}",
                ),
            ],
        ]
        reply_markup = InlineKeyboardMarkup(keyboard)

        async with app:
            await app.bot.send_message(
                chat_id=self.config.telegram_chat_id,
                text=request.format_message(self.config.hostname),
                parse_mode="Markdown",
                reply_markup=reply_markup,
            )

            # Start polling to receive callback queries
            await app.updater.start_polling()  # type: ignore[union-attr]
            await app.start()

            try:
                await asyncio.wait_for(self.decision_event.wait(), timeout=300)
            except asyncio.TimeoutError:
                self.decision = "deny"

            await app.updater.stop()  # type: ignore[union-attr]
            await app.stop()

        return self.decision or "deny"


def create_hook_response(decision: str) -> dict[str, Any]:
    """Create the response JSON for Claude Code hook."""
    behavior = "allow" if decision == "allow" else "deny"
    return {
        "hookSpecificOutput": {
            "hookEventName": "PermissionRequest",
            "decision": {
                "behavior": behavior
            }
        }
    }


async def async_main() -> None:
    """Async main entry point for the hook handler."""
    input_data = json.loads(sys.stdin.read())

    config = Config.load()
    request = PermissionRequest.from_hook_input(input_data)
    handler = DecisionHandler(config)

    decision = await handler.send_request_and_wait(request)
    response = create_hook_response(decision)

    print(json.dumps(response))


def main() -> None:
    """Main entry point for the hook handler."""
    asyncio.run(async_main())


if __name__ == "__main__":
    main()
