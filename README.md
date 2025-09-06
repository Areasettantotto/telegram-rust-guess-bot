# telegram-rust-guess-bot

A small Telegram bot written in Rust that implements a per-user "guess the number" game and loads localized message templates from JSON files in the `messages/` directory.

This README documents how to configure, extend, run, test and deploy the bot.

## Quick start

1. Install Rust (stable toolchain) via https://rustup.rs
2. Create a bot with @BotFather and copy the token.
3. Create a `.env` at the project root or export these variables:

```text
TELOXIDE_TOKEN=YOUR_TOKEN
DEFAULT_LANG=en
GAME_MIN=1
GAME_MAX=100
GAME_ATTEMPTS=5
SEEN_WELCOME_TTL_SECS=2592000
```

4. Run locally:

```bash
cargo run --release
```

## Messages / i18n

All user-facing text is stored in `messages/*.json`. Each file should be named using the language tag stem (for example `en.json`, `it.json`). On startup the bot loads every `*.json` in that directory and accepts files whose stem is recognized by the builtin `parse_lang` logic (the project includes `en`, `it`, `ar`, `ru`, `zh`).

If you prefer fully dynamic tags, we can change the loader to accept any stem and key messages by string tag.

### Required keys

Each JSON file must include these keys (all values are strings). Tests verify they are present and non-empty:

- `cannot_start`
- `cannot_guess`
- `game_started`
- `config`
- `welcome_prompt`
- `no_attempts`
- `revealed`
- `too_low`
- `too_high`
- `lang_set_user`
- `lang_set_chat`
- `lang_invalid`
- `pong`
- `not_started_prompt`
- `current_language_label`
- `language_name`
- `success_correct`

### Example `messages/en.json`

```json
{
  "cannot_start": "I can't start a game for channels or messages without a user.",
  "cannot_guess": "I can't handle guesses without a user.",
  "game_started": "🎯 Game started for you! Guess a number between {min} and {max}...",
  "config": "Current configuration: min = {min}, max = {max}, attempts = {attempts}",
  "welcome_prompt": "Hi {name}! Use /gioco to start your personal game.",
  "no_attempts": "No attempts left. Use /gioco to restart.",
  "revealed": "❌ You've run out of attempts. The number was {target}.",
  "too_low": "Too low. Attempts left: {attempts}",
  "too_high": "Too high. Attempts left: {attempts}",
  "lang_set_user": "Your language preference was set.",
  "lang_set_chat": "Chat language preference was set.",
  "lang_invalid": "Invalid usage. Examples: `/lang en`",
  "pong": "pong",
  "not_started_prompt": "You don't have an active game yet. Use /gioco to start.",
  "current_language_label": "Current language:",
  "language_name": "English",
  "success_correct": "✅ Correct! You guessed it. Game reset."
}
```

## /lang command

Usage examples:

- `/lang` — show the effective language for the current user in this chat.
- `/lang <tag>` — set your personal language in this chat (e.g. `/lang en`).
- `/lang chat <tag>` — set the chat default language (affects everyone unless they have a personal override).

Per-user preferences override the chat default.

## Welcome prompt persistence

The bot persists which users have seen the welcome prompt to `data/seen_welcome.json`. Entries are stored as `"<chat_id>:<user_id>" -> unix_timestamp`. Records older than `SEEN_WELCOME_TTL_SECS` (default 2,592,000 = 30 days) are considered expired and the welcome will be shown again.

If disk read/write fails the bot falls back to in-memory behavior so it remains usable.

## Configuration (environment variables)

- `TELOXIDE_TOKEN` — required Telegram bot token.
- `DEFAULT_LANG` — default language tag (e.g. `en`).
- `GAME_MIN`, `GAME_MAX`, `GAME_ATTEMPTS` — game parameters.
- `SEEN_WELCOME_TTL_SECS` — welcome TTL in seconds.

The project uses `dotenvy` to load a `.env` file if present.

## Running and debugging

- Start locally:

```bash
cargo run --release
```

- Enable backtraces:

```bash
RUST_BACKTRACE=1 cargo run
```

- Format and lint:

```bash
cargo fmt
cargo clippy
```

## Tests

- Run all tests:

```bash
cargo test
```

- Run unit tests only:

```bash
cargo test --lib
```

- Run a specific integration test file:

```bash
cargo test --test lang_and_rand
```

Tests are written to avoid network/Telegram calls. If you add tests that interact with Telegram, mock or stub network calls for CI.

## Deployment notes

The repo includes `scripts/` helper scripts and `dist/telegram-bot.service` systemd unit template.

Recommended patterns:

1. Build and run under a dedicated non-root user (recommended). Install Rust for that user or deploy built artifacts.
2. Build in CI and deploy the release binary to the server; run it under systemd.

Troubleshooting:

- If `git` reports "detected dubious ownership" when running as root, run git as the repo owner or add the repo path to `safe.directory`, or use a dedicated deploy user.
- If systemd fails with `status=217/USER`, ensure the `User=` in the unit exists or change to a valid user.

## Commands summary

Start locally:

```bash
cargo run --release
```

Run tests:

```bash
cargo test
```

Create the .env file from the .env.example file.

```text
## Game configuration
GAME_MIN=1
GAME_MAX=100
GAME_ATTEMPTS=5

# Default language for messages (en or it)
DEFAULT_LANG=it

# Telegram token from @BotFather
TELOXIDE_TOKEN=123456:ABCDEF_your_token

# Logging
RUST_LOG=info

# Welcome persistence TTL in seconds (optional). Default: 2592000 (30 days)
# Use a small value for local testing, e.g. 60
SEEN_WELCOME_TTL_SECS=2592000
```

## Contributing

Contributions welcome. Please keep changes small and add tests for changed behaviour.

## License

Add a license (MIT or Apache-2.0) before publishing; I can add an `LICENSE` file if you want.
