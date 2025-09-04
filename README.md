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

## Running tests

This repository includes both unit and integration tests. The integration tests live under the `tests/` directory and import the library in `src/lib.rs`.

Run the full test suite with:

```bash
cargo test
```

Run only the integration tests:

```bash
cargo test --test lang_and_rand
```

Notes:
- Tests require no external services (they use the library types directly).
- If you add tests that call `run_bot()` or interact with Telegram you should mock or stub network interactions to avoid hitting the real API during CI.
- To run a single test function, use `cargo test <test_name>`.

Unit vs integration tests

- Run library unit tests (tests inside `src/` with `#[cfg(test)]`):

```bash
cargo test --lib
```

- Run a specific integration test file under `tests/` (example `lang_and_rand`):

```bash
cargo test --test lang_and_rand
```

These commands let you execute only the unit tests or only the named integration test without running the entire suite.

- If the bot does not respond in groups, check the bot privacy settings in @BotFather (Privacy Mode).

## Language
The bot supports English (`en`) and Italian (`it`) for user-facing messages. By default the bot uses English. To change the default language, set `DEFAULT_LANG` in your `.env` or export it in your shell:

```text
DEFAULT_LANG=it
# or
DEFAULT_LANG=en
```

This affects the phrasing of replies such as the welcome prompt, hints after guesses, and configuration output.

### Adding languages

You can add more languages by placing a JSON file in the `messages/` directory. The loader maps the file name (the file stem) to the language tag the bot understands. For example:

- `messages/en.json` -> `en`
- `messages/it.json` -> `it`
- `messages/ar.json` -> `ar`
- `messages/ru.json` -> `ru`
- `messages/zh.json` -> `zh`

Rules and notes:

- Use the file stem as the language tag (lowercase). The code calls `parse_lang(stem)` to recognize the tag.
- Files must be valid JSON matching the shape of the existing `messages/*.json` files (keys like `game_started`, `too_low`, `lang_set_user`, etc.).
- On startup the bot loads every `*.json` file under `messages/`. If a file fails to parse or is missing, the bot falls back to default messages (English).
- To add a new language not currently recognized by `parse_lang`, either add the corresponding variant in the code or use the existing supported tags. If you prefer fully dynamic tags, we can refactor to key the map by string tags instead of an enum.

After adding or updating files, restart the bot to pick up the new languages. Users can then set their language with `/lang <tag>` (for example `/lang ru`).

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

## Deploy via SSH

Practical example for deploying on a remote server via SSH:

- Connect to the server:

```bash
ssh root@your_ip
```

- Go to the bot directory and update the repository:

```bash
cd /opt/telegram-bot
git pull origin master
```

- Make the deploy script executable (if it isn't already) and run it:

```bash
chmod +x deploy-bot.sh
./deploy-bot.sh
```

Typical script output shows that changes are pulled, the `release` profile is built, and the systemd service is restarted:

```
⬇️  Pulling latest changes from GitHub...
⚙️  Building bot...
  Finished `release` profile [optimized] target(s) in 0.13s
🔄 Restarting systemd service...
📡 Bot status:
✅ Bot active and running
```

Useful notes:
- Make sure the server has a `TELOXIDE_TOKEN` environment variable or a `.env` file with the bot token.
- The script assumes a systemd service is already set up to manage the bot process; the service name may vary (e.g. `telegram-bot.service`).
- If your repo's main branch is `main`, use `git pull origin main`.

