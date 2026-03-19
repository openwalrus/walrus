# crabtalk-telegram

Telegram bot gateway for [Crabtalk](https://github.com/crabtalk/crabtalk) agents.

Connects a Telegram bot to Crabtalk agents via the crabtalk daemon, streaming
AI responses with edit-in-place updates.

## Install

```
cargo install crabtalk-telegram
```

Or via the Crabtalk hub:

```
crabtalk hub install crabtalk/telegram
```

## Usage

```
crabtalk-telegram serve --daemon /path/to/daemon.sock --config '{"telegram":{"token":"BOT_TOKEN"}}'
```

The bot token is obtained from [@BotFather](https://t.me/BotFather) on Telegram.

### Restrict to specific users

Add `allowed_users` with a list of Telegram user IDs. When set, the bot
silently ignores messages from anyone not on the list:

```toml
[services.telegram]
crate = "crabtalk-telegram"
kind = "gateway"
config = { telegram = { token = "BOT_TOKEN", allowed_users = [123456789] } }
```

Omit `allowed_users` (or leave it empty) to allow everyone.

## Features

- Streams AI responses with real-time edit-in-place updates
- MarkdownV2 formatting with plain-text fallback
- Hub install/uninstall commands via `/hub install|uninstall <package>`
- Attachment forwarding (images, audio, video, files)
- Automatic session management with error recovery

## License

MIT
