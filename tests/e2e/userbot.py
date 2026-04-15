"""
MTProto Userbot Helper for E2E Tests

Uses Telethon to interact with Telegram as a real user.
This allows programmatically clicking inline keyboard buttons
that the bot sends during permission requests.
"""

import asyncio
import os
from typing import Optional

from telethon import TelegramClient, events
from telethon.sessions import StringSession
from telethon.tl.types import Message
from telethon.tl.custom import Button


class TelegramUserbot:
    """Wrapper around Telethon client for test automation."""

    def __init__(
        self,
        api_id: int,
        api_hash: str,
        session_string: Optional[str] = None,
        phone: Optional[str] = None,
    ):
        self.api_id = api_id
        self.api_hash = api_hash
        self.phone = phone

        # Use session string if provided, otherwise create new session
        if session_string:
            self.client = TelegramClient(
                StringSession(session_string), api_id, api_hash
            )
        else:
            self.client = TelegramClient(StringSession(), api_id, api_hash)

        self._bot_id: Optional[int] = None
        self._update_task: Optional[asyncio.Task] = None

    @classmethod
    def from_env(cls) -> "TelegramUserbot":
        """Create userbot from environment variables."""
        api_id = int(os.environ["TELEGRAM_API_ID"])
        api_hash = os.environ["TELEGRAM_API_HASH"]
        session_string = os.environ.get("TELEGRAM_SESSION")
        phone = os.environ.get("TELEGRAM_PHONE")

        return cls(api_id, api_hash, session_string, phone)

    async def start(self, phone: Optional[str] = None) -> str:
        """
        Start the client and return session string.

        If no session exists, will prompt for phone/code interactively.
        """
        phone = phone or self.phone
        await self.client.start(phone=phone)

        # Catch up on any pending updates to sync state
        await self.client.catch_up()

        return self.client.session.save()

    async def disconnect(self):
        """Disconnect the client."""
        if self._update_task:
            self._update_task.cancel()
            try:
                await self._update_task
            except asyncio.CancelledError:
                pass
        await self.client.disconnect()

    def set_bot_id(self, bot_id: int):
        """Set the bot ID to watch for messages from."""
        self._bot_id = bot_id

    async def wait_for_message(
        self,
        bot_id: Optional[int] = None,
        timeout: float = 30.0,
        contains: Optional[str] = None,
    ) -> Message:
        """
        Wait for a new message from the specified bot by polling.

        Args:
            bot_id: Bot user ID to watch for (uses default if not specified)
            timeout: Maximum time to wait in seconds
            contains: Optional substring that must be in the message

        Returns:
            The received Message object

        Raises:
            TimeoutError: If no matching message received within timeout
        """
        bot_id = bot_id or self._bot_id
        if not bot_id:
            raise ValueError("bot_id must be specified or set via set_bot_id()")

        # Get current latest message ID to detect new messages
        latest_id = 0
        async for msg in self.client.iter_messages(bot_id, limit=1):
            latest_id = msg.id
            break

        # Poll for new messages
        start_time = asyncio.get_event_loop().time()
        poll_interval = 0.5  # Poll every 500ms

        while True:
            elapsed = asyncio.get_event_loop().time() - start_time
            if elapsed >= timeout:
                raise TimeoutError(
                    f"No message from bot {bot_id} received within {timeout}s"
                )

            # Check for new messages since latest_id
            async for msg in self.client.iter_messages(bot_id, limit=5, min_id=latest_id):
                if msg.sender_id == bot_id:
                    # Check optional content filter
                    if contains is None or contains in (msg.text or ""):
                        return msg

            await asyncio.sleep(poll_interval)

    async def get_last_message_from_bot(
        self, bot_id: Optional[int] = None, limit: int = 10
    ) -> Optional[Message]:
        """
        Get the most recent message from the bot.

        Useful for checking messages that were already sent.
        """
        bot_id = bot_id or self._bot_id
        if not bot_id:
            raise ValueError("bot_id must be specified or set via set_bot_id()")

        async for message in self.client.iter_messages(bot_id, limit=limit):
            if message.sender_id == bot_id:
                return message

        return None

    async def click_button(
        self, message: Message, button_text: str
    ) -> Optional[Message]:
        """
        Click an inline keyboard button by its text.

        Args:
            message: The message containing the inline keyboard
            button_text: Partial text to match on the button

        Returns:
            The response message (if any)

        Raises:
            ValueError: If button not found
        """
        if not message.buttons:
            raise ValueError("Message has no inline buttons")

        # Find the button
        for row in message.buttons:
            for button in row:
                if button_text.lower() in button.text.lower():
                    # Click the button
                    result = await button.click()
                    return result

        raise ValueError(f"Button containing '{button_text}' not found")

    async def click_button_by_index(
        self, message: Message, row: int = 0, col: int = 0
    ) -> Optional[Message]:
        """
        Click an inline keyboard button by its position.

        Args:
            message: The message containing the inline keyboard
            row: Row index (0-based)
            col: Column index (0-based)

        Returns:
            The response message (if any)
        """
        if not message.buttons:
            raise ValueError("Message has no inline buttons")

        try:
            button = message.buttons[row][col]
            result = await button.click()
            return result
        except IndexError:
            raise ValueError(f"Button at position ({row}, {col}) not found")

    async def send_message(self, entity, text: str) -> Message:
        """Send a message to an entity (user, chat, etc)."""
        return await self.client.send_message(entity, text)


async def generate_session_string():
    """
    Interactive helper to generate a session string.

    Run this locally (not in Docker) to get SMS code.
    """
    api_id = int(input("Enter API ID: "))
    api_hash = input("Enter API Hash: ")
    phone = input("Enter phone number (with country code): ")

    client = TelegramClient(StringSession(), api_id, api_hash)

    await client.start(phone=phone)

    session_string = client.session.save()
    print("\n" + "=" * 50)
    print("SESSION STRING (save this!):")
    print("=" * 50)
    print(session_string)
    print("=" * 50)

    await client.disconnect()
    return session_string


if __name__ == "__main__":
    # Run this script directly to generate a session string
    asyncio.run(generate_session_string())
