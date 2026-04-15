#!/usr/bin/env python3
"""Generate session string with code provided as argument."""
import asyncio
import sys
from telethon import TelegramClient
from telethon.sessions import StringSession

API_ID = 35365500
API_HASH = '4ea529afa20bd8ba01d19175ad13135c'
PHONE = '+821042792134'

async def main():
    if len(sys.argv) < 2:
        print("Usage: python gen_session_with_code.py <verification_code>")
        sys.exit(1)

    code = sys.argv[1]

    client = TelegramClient(StringSession(), API_ID, API_HASH)

    # Define code callback that returns the provided code
    def code_callback():
        return code

    try:
        await client.start(phone=PHONE, code_callback=code_callback)

        print("\n" + "="*60)
        print("SESSION STRING:")
        print("="*60)
        print(client.session.save())
        print("="*60 + "\n")

    finally:
        await client.disconnect()

if __name__ == "__main__":
    asyncio.run(main())
