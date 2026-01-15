//! Long-running Telegram bot for /start, /help, /status commands.

use crate::config::Config;
use crate::telegram::escape_markdown;
use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommands;

/// Available bot commands.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Show your chat ID for configuration")]
    Start,
    #[command(description = "Show help and setup instructions")]
    Help,
    #[command(description = "Check bot status")]
    Status,
}

/// Handle the /start command.
async fn start_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let text = format!(
        "👋 *Welcome to Claude Code Telegram Bot\\!*\n\n\
        Your chat ID is: `{}`\n\n\
        Add this to your configuration file:\n\
        📁 `~/.claude/telegram_hook.json`\n\
        ```json\n\
        {{\n\
          \"telegram_bot_token\": \"YOUR_BOT_TOKEN\",\n\
          \"telegram_chat_id\": \"{}\"\n\
        }}\n\
        ```",
        chat_id, chat_id
    );

    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

/// Handle the /help command.
async fn help_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    let text = r#"📖 *Claude Code Telegram Bot Help*

This bot integrates with Claude Code to handle permission requests remotely\.

*Setup:*
1\. Get your chat ID with /start
2\. Create config at `~/.claude/telegram_hook.json`
3\. Add hooks to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PermissionRequest": [{
      "matcher": {"tools": ["Bash", "Edit", "Write"]},
      "hooks": [{"type": "command", "command": "claude-code-telegram hook"}]
    }],
    "Stop": [{
      "matcher": {},
      "hooks": [{"type": "command", "command": "claude-code-telegram stop"}]
    }]
  }
}
```

*Features:*
• Permission request notifications
• Allow/Deny/Always Allow buttons
• Job completion notifications
• Multi\-machine hostname display

*Commands:*
/start \- Show your chat ID
/help \- Show this help
/status \- Check bot status"#;

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

/// Handle the /status command.
async fn status_handler(bot: Bot, msg: Message, config: &Config) -> ResponseResult<()> {
    let text = format!(
        "✅ *Bot Status: Online*\n\n\
        🖥️ *Host:* `{}`\n\
        💬 *Chat ID:* `{}`",
        escape_markdown(&config.hostname),
        msg.chat.id
    );

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

/// Main entry point for the bot.
pub async fn run() -> Result<()> {
    let config = Config::load(None)?;

    let telegram_config = config
        .telegram
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Telegram configuration required for bot command"))?;

    let bot = Bot::new(&telegram_config.bot_token);

    tracing::info!("Starting Claude Code Telegram Bot...");

    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint({
            let config = config.clone();
            move |bot: Bot, msg: Message, cmd: Command| {
                let config = config.clone();
                async move {
                    match cmd {
                        Command::Start => start_handler(bot, msg).await,
                        Command::Help => help_handler(bot, msg).await,
                        Command::Status => status_handler(bot, msg, &config).await,
                    }
                }
            }
        });

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
