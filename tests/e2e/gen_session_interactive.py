#!/usr/bin/env python3
"""Interactive session generator that keeps connection open."""
import asyncio
from telethon import TelegramClient
from telethon.sessions import StringSession

API_ID = 35365500
API_HASH = '4ea529afa20bd8ba01d19175ad13135c'
PHONE = '+821042792134'

async def main():
    client = TelegramClient(StringSession(), API_ID, API_HASH)

    # This will handle the entire auth flow interactively
    await client.start(phone=PHONE)

    print("\n" + "="*60)
    print("SESSION STRING:")
    print("="*60)
    print(client.session.save())
    print("="*60 + "\n")

    await client.disconnect()

if __name__ == "__main__":
    asyncio.run(main())
