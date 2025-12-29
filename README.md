# Walky Tasky Bot

A Telegram bot for task scheduling and reminders, built with Rust.

## Features

- Create and schedule tasks via Telegram commands
- PostgreSQL persistence with automatic migrations
- Retry mechanism with exponential backoff for failed tasks
- Extensible action executor system

## Project Structure

```
walky_tasky_bot/
├── bot/          # Telegram bot implementation using teloxide
├── scheduler/    # Task scheduling engine with storage backends
```

## Requirements

- Rust 2024 edition
- PostgreSQL database
- Telegram Bot Token (from [@BotFather](https://t.me/botfather))

## Configuration

Set the following environment variables:

| Variable         | Default     | Description                   |
| ---------------- | ----------- | ----------------------------- |
| `TELOXIDE_TOKEN` | -           | Telegram bot token (required) |
| `DB_USER`        | `postgres`  | Database username             |
| `DB_PASSWORD`    | `postgres`  | Database password             |
| `DB_HOST`        | `localhost` | Database host                 |
| `DB_PORT`        | `5432`      | Database port                 |
| `DB_NAME`        | `wt_db`     | Database name                 |

## Running

```bash
# Set your Telegram bot token
export TELOXIDE_TOKEN="your_bot_token"

# Run the bot
cargo run -p bot
```

## Bot Commands

- `/help` - Show available commands
- `/novi_zadatak` - Create a new task

## Development

```bash
# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features
```

## License

MIT
