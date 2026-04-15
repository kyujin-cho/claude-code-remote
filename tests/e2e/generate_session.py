#!/usr/bin/env python3
"""
Generate Telegram session string for E2E tests.

Run this script to authenticate with Telegram and get a session string
that can be used in CI/CD without re-authenticating.

Usage:
    python generate_session.py
"""

import asyncio
import sys

from telethon import TelegramClient
from telethon.sessions import StringSession


async def main():
    print("=" * 50)
    print("Telegram Session String Generator")
    print("=" * 50)
    print()

    # Get credentials
    api_id = input("Enter API ID: ").strip()
    api_hash = input("Enter API Hash: ").strip()
    phone = input("Enter phone number (e.g., +821012345678): ").strip()

    if not api_id or not api_hash or not phone:
        print("Error: All fields are required")
        sys.exit(1)

    api_id = int(api_id)

    print()
    print("Connecting to Telegram...")

    client = TelegramClient(StringSession(), api_id, api_hash)

    await client.start(phone=phone)

    # Get the session string
    session_string = client.session.save()

    print()
    print("=" * 50)
    print("SUCCESS! Your session string:")
    print("=" * 50)
    print()
    print(session_string)
    print()
    print("=" * 50)
    print()
    print("Save this string securely. You can use it as:")
    print("  export TELEGRAM_SESSION='<session_string>'")
    print()
    print("Or add it to your .env file.")

    await client.disconnect()


if __name__ == "__main__":
    asyncio.run(main())
