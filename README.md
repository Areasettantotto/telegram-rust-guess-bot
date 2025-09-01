# telegram-bot-rust

Minimal Telegram bot written in Rust that implements a simple "guess the number" game per chat.

This repository demonstrates a practical integration with Telegram using the `teloxide` crate, `tokio` for the async runtime, and `rand` for random number generation.

## Main files
- `src/main.rs` — bot implementation and game logic
- `Cargo.toml` — dependencies

## Requirements
- Rust toolchain (stable)
- A Telegram bot token (create one via @BotFather)

## Quick setup
1. Create the bot with BotFather on Telegram and copy the token provided.
2. Add the token as an environment variable or in a `.env` file at the project root.

Option A — `.env` file (recommended for development):

```text
TELOXIDE_TOKEN=123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11
```
You can also configure the game behavior using the following optional environment variables:

```text
# Minimum target number (inclusive). Default: 1
GAME_MIN=1
# Maximum target number (inclusive). Default: 100
GAME_MAX=100
# Number of attempts per player. Default: 5
GAME_ATTEMPTS=5
# Default language for bot messages: 'en' or 'it' (default: en)
DEFAULT_LANG=en
```

Option B — export in your shell:

```bash
export TELOXIDE_TOKEN="123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"
```

Note: the project automatically loads `.env` using `dotenvy` if present.

## Run the bot

From the project folder:

```bash
cargo run
```

You can also try the game right away by opening Telegram and searching for the public bot @RustGuessBot.

Supported commands (in chat with the bot):
- `/ping` — replies `pong`
- `/gioco` — starts (or restarts) the game: starts a personal game for the user, the bot chooses a number in the configured range and gives you the configured number of attempts; send numbers to guess.
- `/config` — shows the current game configuration (GAME_MIN, GAME_MAX, GAME_ATTEMPTS)

- `/lang` — manage language preferences (per-user or per-chat). Examples below.

## Debug & troubleshooting
- If you get a panic on startup: ensure `TELOXIDE_TOKEN` is set (`echo $TELOXIDE_TOKEN` or `cat .env`).
- For detailed stack traces:

```bash
RUST_BACKTRACE=1 cargo run
```

- For formatting and linting:

```bash
cargo fmt
cargo clippy
```

- If the bot does not respond in groups, check the bot privacy settings in @BotFather (Privacy Mode).

## Language
The bot supports English (`en`) and Italian (`it`) for user-facing messages. By default the bot uses English. To change the default language, set `DEFAULT_LANG` in your `.env` or export it in your shell:

```text
DEFAULT_LANG=it
# or
DEFAULT_LANG=en
```

This affects the phrasing of replies such as the welcome prompt, hints after guesses, and configuration output.

### /lang command
The `/lang` command lets users control language preferences.

Usage examples:

- `/lang en` — sets your personal language to English (applies to you in the current chat).
- `/lang it` — sets your personal language to Italian.
- `/lang chat en` — sets the default language for the whole chat to English (affects every user in that chat unless they set a personal override).
- `/lang` — shows the effective language for the current user in this chat.

Notes:
- Per-user preferences take precedence over the chat default.
- Any user can set their own language. Changing the chat language affects everyone in the chat.

## Structure & logic
- Game state is kept in-memory per user per chat in `SharedState` (non-persistent).
- The `rand_in_range` function uses the `rand` crate to sample the target number.

## Contributing
- Pull requests and issues are welcome. Keep changes small and add tests where appropriate.

## License
Add your preferred license here (e.g. MIT, Apache-2.0) before publishing on GitHub.

---

If you want, I can also add a `LICENSE` (MIT) and a `.gitignore` tailored for Rust projects before you publish on GitHub.

---

README updated to reflect new `.env` options: `GAME_MIN`, `GAME_MAX`, `GAME_ATTEMPTS`.
