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

Supported commands (in chat with the bot):
- `/ping` — replies `pong`
- `/gioco` — starts (or restarts) the game: the bot chooses a number between 1 and 100 and gives you 5 attempts; send numbers to guess.

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

## Structure & logic
- Game state per chat is kept in-memory in `SharedState` (non-persistent).
- The `rand_in_range` function uses the `rand` crate to sample the target number.

## Contributing
- Pull requests and issues are welcome. Keep changes small and add tests where appropriate.

## License
Add your preferred license here (e.g. MIT, Apache-2.0) before publishing on GitHub.

---

If you want, I can also add a `LICENSE` (MIT) and a `.gitignore` tailored for Rust projects before you publish on GitHub.
