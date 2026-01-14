#!/usr/bin/env python3
"""CLI entry point for Claude Code Telegram integration.

Provides subcommands for different hook handlers and the bot.
"""

import argparse
import sys


def main() -> None:
    """Main CLI entry point with subcommands."""
    parser = argparse.ArgumentParser(
        prog="claude-code-telegram",
        description="Claude Code hook & Telegram Bot integration",
    )
    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # Hook subcommand (PermissionRequest)
    subparsers.add_parser(
        "hook",
        help="Handle PermissionRequest hooks (reads from stdin)",
    )

    # Stop subcommand
    subparsers.add_parser(
        "stop",
        help="Handle Stop hooks for job completion notifications (reads from stdin)",
    )

    # Bot subcommand
    subparsers.add_parser(
        "bot",
        help="Run the Telegram bot for /start and /help commands",
    )

    args = parser.parse_args()

    if args.command == "hook":
        from claude_code_telegram.hook_handler import main as hook_main

        hook_main()
    elif args.command == "stop":
        from claude_code_telegram.stop_handler import main as stop_main

        stop_main()
    elif args.command == "bot":
        from claude_code_telegram.bot import main as bot_main

        bot_main()
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
