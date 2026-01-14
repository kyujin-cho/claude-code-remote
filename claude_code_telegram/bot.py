#!/usr/bin/env python3
"""Telegram bot for receiving permission decisions.

This is a standalone bot that can run continuously and handle
permission requests from the hook handler.
"""

import logging
from typing import Any

from telegram import Update
from telegram.ext import (
    Application,
    CommandHandler,
    ContextTypes,
)

from claude_code_telegram.config import Config

logging.basicConfig(
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    level=logging.INFO,
)
logger = logging.getLogger(__name__)


async def start_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle the /start command."""
    if update.effective_chat is None:
        return

    chat_id = update.effective_chat.id
    await update.message.reply_text(  # type: ignore[union-attr]
        f"👋 Hello! I'm the Claude Code Decision Bot.\n\n"
        f"Your chat ID is: `{chat_id}`\n\n"
        f"Add this to your `.env` file as `TELEGRAM_CHAT_ID` to receive "
        f"permission requests from Claude Code.",
        parse_mode="Markdown",
    )


async def help_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle the /help command."""
    help_text = """
*Claude Code Decision Bot*

This bot receives permission requests from Claude Code and lets you approve or deny them.

*Setup:*
1. Copy your chat ID from the /start message
2. Add it to your `.env` file
3. Configure Claude Code hooks to use `hook_handler.py`

*Commands:*
/start - Get your chat ID
/help - Show this help message
/status - Check bot status
    """
    await update.message.reply_text(help_text, parse_mode="Markdown")  # type: ignore[union-attr]


async def status_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle the /status command."""
    await update.message.reply_text(  # type: ignore[union-attr]
        "✅ Bot is running and ready to receive permission requests."
    )


def create_bot(config: Config) -> Application[Any, Any, Any, Any, Any, Any]:
    """Create and configure the Telegram bot application."""
    app = Application.builder().token(config.telegram_bot_token).build()

    app.add_handler(CommandHandler("start", start_command))
    app.add_handler(CommandHandler("help", help_command))
    app.add_handler(CommandHandler("status", status_command))

    return app


def main() -> None:
    """Run the bot."""
    config = Config.from_env()
    app = create_bot(config)

    logger.info("Starting bot...")
    app.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == "__main__":
    main()
