# Telegram Guess Bot (Rust)

This repository contains a simple Telegram bot written in Rust that lets users play a "guess the number" game.

## Game rules
- Each game starts with `GAME_ATTEMPTS` attempts (configurable via env, e.g. 10).
- Every win reduces the starting attempts for the next game by 1 for that same user (never below 1). This reduction can be applied across multiple consecutive wins, but it's limited.
- Scaling limit: the progressive decrement can be applied at most `NUMBER_ATTEMPTS` consecutive wins (default: 3). In practice the number of attempts can decrease stepwise (e.g. 10 → 9 → 8 → 7) up to `NUMBER_ATTEMPTS` times; after that point further automatic decrements are paused until a reset condition occurs (see below).
- Reset: when a user exhausts their attempts without guessing, their "miss streak" (consecutive failed games) is incremented. When the miss streak reaches `NUMBER_ATTEMPTS`, the user's starting attempts are reset to `GAME_ATTEMPTS` and the miss streak is reset to 0.

Example (suggested defaults):
- `GAME_ATTEMPTS = 10`
- `NUMBER_ATTEMPTS = 3` (maximum 3 consecutive decrements)

Typical sequence:
1. Game 1: user wins → next game starts with 9 attempts (10 → 9).
2. Game 2: user wins again → next game starts with 8 attempts (9 → 8).
3. Game 3: user wins again → next game starts with 7 attempts (8 → 7). Three consecutive decrements have been applied; no further automatic decrements will be applied beyond this limit.
4. If the user subsequently loses for `NUMBER_ATTEMPTS` consecutive games (miss streak = 3), the starting attempts are reset to `GAME_ATTEMPTS` (10) and the miss streak is cleared.

## Messages and localization
All user-facing text is stored in `messages/*.json`. The success message includes the `{next_attempts}` placeholder, which will be replaced with the number of attempts for the next game. Make sure translations include `{next_attempts}` where appropriate.

## Persistence
The bot persists two maps on disk under the `data/` folder:
- `data/user_start_attempts.json` — map `"<chat_id>:<user_id>" -> start_attempts` indicating how many attempts the next game will start with for that user.
- `data/user_miss_streaks.json` — map `"<chat_id>:<user_id>" -> consecutive_misses` (count of consecutive games lost).

These files are loaded at startup and updated on a best-effort basis during runtime (I/O errors are currently ignored so the bot remains usable if disk writes fail).

## Relevant commands
- `/gioco` — start (or restart) your personal game.
- `/lang` — language management.
- `/config` — display current configuration.
- `/reset_starts` — admin command that clears `user_start_attempts.json`. Only the user configured in `BOT_OWNER_ID` can run this command.

## Environment variables
- `GAME_MIN` — minimum of the number range (default: 1)
- `GAME_MAX` — maximum of the number range (default: 100)
- `GAME_ATTEMPTS` — initial attempts for a full game (default: 5)
- `NUMBER_ATTEMPTS` — how many consecutive events are considered for scaling/reset (default: 3)
- `DEFAULT_LANG` — default language tag (e.g. `en`)
- `BOT_OWNER_ID` — Telegram user ID allowed to run `/reset_starts`

The project uses `dotenvy` to read a `.env` file when present.

## Tests and development
- Run tests:

```bash
cargo test
```

- Unit and integration tests avoid making network/Telegram calls wherever possible. Tests that would interact with Telegram require mocking.

### Tests (files and purpose)

This project includes several tests located in `tests/` and in the library's `#[cfg(test)]` module. Below is a short summary of each file and its intent. Several tests were recently added or updated to reflect the current rule that "any win decrements the next game's start attempts by 1".

- `tests/decrement_sequence.rs`
  - Verifies consecutive wins decrement stored `start_attempts` (e.g. 10 → 9 → 8 → 7) and that any win decrements by 1.

- `tests/game_restart_threshold.rs`
  - Exercises `next_attempts_after_win` helper; updated to the current behavior where every win decrements by 1.

- `tests/messages_load.rs` and `tests/messages_keys.rs`
  - Validate that `messages/*.json` load and that required message keys exist.

- `tests/lang_detection.rs` and `tests/lang_and_rand.rs`
  - Validate language detection heuristics and random number helper behavior.

- `tests/full_flow_reset.rs`
  - Simulates the reset-on-miss flow by manipulating the persisted maps and verifying `user_start_attempts` resets to `GAME_ATTEMPTS` after `NUMBER_ATTEMPTS` consecutive failures.

Library (internal) tests in `src/lib.rs`:

- `seen_welcome_ttl_renewal` — ensures welcome TTL and persistence behavior.
- `simulate_start_quickwin_restart_cycle` — ensures a quick win decrements and persists start attempts.
- `simulate_consecutive_losses_reset` — verifies miss-streak accumulation and reset of start attempts after threshold.
- `simulate_two_quick_wins_decrement_twice` — ensures two consecutive wins decrement persisted start attempts twice (10 → 9 → 8).

Notes:
- Tests use unique temporary filenames (PID + nanoseconds) to avoid collisions when running in parallel.
- Some tests were updated to match the current behavior: the helper `next_attempts_after_win` now always returns `previous_start_attempts - 1` (clamped to 1) for wins; tests that expected no decrement for "slow" wins were adjusted accordingly.

## Running and debugging


```bash
cargo run --release
```

## Changelog — 05 October 2025

Summary of changes applied on 05 October 2025:

- Placeholders and messages
  - `success_correct` and `revealed` now support `{number_attempts}` to show the remaining chances before a reset.
  - `{next_attempts}` now represents the maximum attempts for the current/next game (derived from the per-user `start_attempts`).
  - `{attempts}` continues to show attempts remaining in the active game.

- Commands
  - `/config` now displays: minimum, maximum, attempts (remaining), `next_attempts` (max for the game) and `number_attempts` (remaining failures before reset).
  - `/reset_starts` was enhanced to also clear `user_miss_streaks` so the remaining-before-reset value returns to the full threshold.

- Persistence and git
  - The `data/` directory is now ignored via `.gitignore` and was removed from version control (files remain on disk).
  - The bot automatically creates and updates `data/user_start_attempts.json` and `data/user_miss_streaks.json` at runtime.

- Tests
  - Integration tests were added/updated in `tests/config_display.rs` to assert correct substitution for `{attempts}`, `{next_attempts}` and `{number_attempts}` using `messages/it.json`.

How to verify locally

1. Run the unit/integration tests:

```bash
cargo test
```

2. Build and run the bot, then interact in Telegram:

```bash
cargo run --release
# or
cargo build --release && ./target/release/telegram-bot-rust
```

3. Manual checks:
- Start a game with `/gioco`, then run `/config` — observe `Tentativi rimasti` and `Numero massimo di tentativi per questa partita` (or their English equivalents).
- Lose a game and check `/config` to see the updated `number_attempts` (remaining before reset).

If you want, I can:
- Split this changelog into individual commit references, or
- Add an explicit section describing the JSON format of `data/*.json` or
- Create a small `scripts/seed_data.sh` to pre-populate `data/` for testing.

- Enable backtraces:

```bash
RUST_BACKTRACE=1 cargo run
```

- Format and lint:

```bash
cargo fmt
cargo clippy
```

## Deployment notes

The repository contains helper scripts and a systemd unit template in `dist/telegram-bot.service`.

Recommended deployment pattern:

1. Build and run under a dedicated non-root user (e.g. `telegrambot`).
2. Prefer building in CI and deploying the release binary to the server; run it under systemd.

If you modify the systemd unit file, remember to run `systemctl daemon-reload` and restart the service.

## Troubleshooting

- If `git` complains about dubious ownership when running as root, run git as the repo owner or add the repo path to `safe.directory`.
- If systemd reports `status=217/USER`, ensure the `User=` in the unit exists.

## Contributing

Contributions welcome. Keep changes small and add tests for behavior changes.

## License

Add a license (MIT or Apache-2.0) before publishing; I can add a `LICENSE` file if you want.
