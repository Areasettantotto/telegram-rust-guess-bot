// telegram-bot-rust
// -----------------
// Minimal Telegram bot in Rust implementing a per-chat "guess the number" game.
// Features:
//  - /ping command (responds with "pong")
//  - /gioco command to start/restart a guessing game (1..100, 5 attempts)
//  - per-chat in-memory game state stored in a shared RwLock
//  - uses teloxide for Telegram integration, tokio for async runtime, rand for RNG
//
// Author: Marco Busato
//

use std::{collections::HashMap, env, sync::Arc};

use anyhow::Result;
use dotenvy::dotenv;
use rand::Rng;
use teloxide::prelude::*;
use tokio::sync::RwLock;

/// State of a single game for a user in a chat
#[derive(Clone, Debug)]
struct GameState {
    target: i32,
    attempts_left: i32,
}

/// Shared application state
struct AppState {
    // key: (chat_id, user_id)
    by_user: HashMap<(i64, u64), GameState>,
    // language preferences
    user_langs: HashMap<(i64, u64), Lang>,
    chat_langs: HashMap<i64, Lang>,
}

type SharedState = Arc<RwLock<AppState>>;

/// Runtime configuration (from environment with sensible defaults)
#[derive(Clone, Debug)]
struct Config {
    min: i32,
    max: i32,
    attempts: i32,
    lang: Lang,
}

type SharedConfig = Arc<Config>;

#[derive(Clone, Copy, Debug)]
enum Lang {
    En,
    It,
}

// Localized message helpers
fn msg_cannot_start(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "I can't start a game for channels or messages without a user.",
        Lang::It => "Non posso avviare una partita per canali o messaggi senza utente.",
    }
}

fn msg_cannot_guess(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "I can't handle guesses without a user.",
        Lang::It => "Non posso gestire congetture senza un utente.",
    }
}

fn msg_game_started(lang: Lang, min: i32, max: i32, attempts: i32) -> String {
    match lang {
        Lang::En => format!(
            "🎯 Game started for you! Guess a number between {} and {}. Attempts left: {}\n",
            min, max, attempts
        ),
        Lang::It => format!(
            "🎯 Gioco avviato per te! Indovina un numero tra {} e {}. Tentativi rimasti: {}\n",
            min, max, attempts
        ),
    }
}

fn msg_config(lang: Lang, min: i32, max: i32, attempts: i32) -> String {
    match lang {
        Lang::En => format!(
            "Current configuration: min = {}, max = {}, attempts = {}",
            min, max, attempts
        ),
        Lang::It => format!(
            "Configurazione corrente: min = {}, max = {}, tentativi = {}",
            min, max, attempts
        ),
    }
}

fn msg_welcome_prompt(lang: Lang, name: &str) -> String {
    match lang {
        Lang::En => format!("Hi {}! Use /gioco to start your personal game.", name),
        Lang::It => format!(
            "Ciao {}! Usa /gioco per iniziare la tua partita personale.",
            name
        ),
    }
}

fn msg_no_attempts(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "No attempts left. Use /gioco to restart.",
        Lang::It => "Nessun tentativo rimasto. Usa /gioco per ricominciare.",
    }
}

fn msg_revealed(lang: Lang, target: i32) -> String {
    match lang {
        Lang::En => format!(
            "❌ You've run out of attempts. The number was {}. Use /gioco to restart.",
            target
        ),
        Lang::It => format!(
            "❌ Hai esaurito i tentativi. Il numero era {}. Usa /gioco per ricominciare.",
            target
        ),
    }
}

// msg_correct was removed because we reset the game and reuse other messages

fn msg_too_low(lang: Lang, attempts: i32) -> String {
    match lang {
        Lang::En => format!("Too low. Attempts left: {}", attempts),
        Lang::It => format!("Troppo basso. Tentativi rimasti: {}", attempts),
    }
}

fn msg_too_high(lang: Lang, attempts: i32) -> String {
    match lang {
        Lang::En => format!("Too high. Attempts left: {}", attempts),
        Lang::It => format!("Troppo alto. Tentativi rimasti: {}", attempts),
    }
}

fn msg_lang_current(_lang: Lang, current: Lang) -> String {
    match current {
        Lang::En => "Current language: English (en)".to_string(),
        Lang::It => "Lingua corrente: Italiano (it)".to_string(),
    }
}

fn msg_lang_set_user(lang: Lang) -> String {
    match lang {
        Lang::En => "Your language preference was set.".to_string(),
        Lang::It => "La tua lingua è stata impostata.".to_string(),
    }
}

fn msg_lang_set_chat(lang: Lang) -> String {
    match lang {
        Lang::En => "Chat language preference was set.".to_string(),
        Lang::It => "La lingua della chat è stata impostata.".to_string(),
    }
}

fn msg_lang_invalid(lang: Lang) -> String {
    match lang {
        Lang::En => "Invalid usage. Examples: `/lang en`, `/lang it`, `/lang chat en`".to_string(),
        Lang::It => "Uso non valido. Esempi: `/lang en`, `/lang it`, `/lang chat en`".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load `.env` file if present so TELOXIDE_TOKEN can be set there.
    dotenv().ok();

    let bot = Bot::from_env();

    // Load runtime configuration from environment (optional)
    let cfg = Config {
        min: env::var("GAME_MIN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        max: env::var("GAME_MAX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
        attempts: env::var("GAME_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5),
        lang: {
            let d = env::var("DEFAULT_LANG")
                .ok()
                .unwrap_or_else(|| "en".to_string());
            if d.to_lowercase().starts_with("it") {
                Lang::It
            } else {
                Lang::En
            }
        },
    };
    let shared_config = Arc::new(cfg);

    // Validate configuration early and fail fast with clear error messages
    if shared_config.min >= shared_config.max {
        anyhow::bail!(
            "Invalid configuration: GAME_MIN ({}) must be less than GAME_MAX ({}).",
            shared_config.min,
            shared_config.max
        );
    }
    if shared_config.attempts <= 0 {
        anyhow::bail!(
            "Invalid configuration: GAME_ATTEMPTS ({}) must be a positive integer.",
            shared_config.attempts
        );
    }
    let state = Arc::new(RwLock::new(AppState {
        by_user: HashMap::new(),
        user_langs: HashMap::new(),
        chat_langs: HashMap::new(),
    }));

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let state = state.clone();
        let shared_config = shared_config.clone();
        async move {
            if let Err(err) = handle_message(&bot, &msg, state, shared_config).await {
                tracing::error!("handler error: {:?}", err);
            }
            respond(())
        }
    })
    .await;

    Ok(())
}

async fn handle_message(
    bot: &Bot,
    msg: &Message,
    state: SharedState,
    config: SharedConfig,
) -> Result<()> {
    // Determine effective language for this message (user override -> chat override -> default)
    let lang = {
        // helper to compute effective lang
        async fn effective_lang(state: &SharedState, msg: &Message, default: Lang) -> Lang {
            let chat_id = msg.chat.id.0;
            let lock = state.read().await;
            if let Some(user) = msg.from.as_ref() {
                let key = (chat_id, user.id.0);
                if let Some(&l) = lock.user_langs.get(&key) {
                    return l;
                }
            }
            if let Some(&l) = lock.chat_langs.get(&chat_id) {
                return l;
            }
            default
        }

        effective_lang(&state, msg, config.lang).await
    };
    if let Some(text) = msg.text() {
        let text = text.trim();

        // Simple commands
        if text.eq_ignore_ascii_case("/ping") {
            bot.send_message(msg.chat.id, "pong").await?;
            return Ok(());
        }

        if text.eq_ignore_ascii_case("/gioco") {
            // Require a user context for per-user games
            let user_id = match msg.from.as_ref().map(|u| u.id.0) {
                Some(id) => id,
                None => {
                    bot.send_message(msg.chat.id, msg_cannot_start(config.lang))
                        .await?;
                    return Ok(());
                }
            };

            let mut lock = state.write().await;
            let key = (msg.chat.id.0, user_id);

            // Reset/replace the game for this user in this chat using configured range/attempts
            let new_game = GameState {
                target: rand_in_range(config.min, config.max),
                attempts_left: config.attempts,
            };
            lock.by_user.insert(key, new_game.clone());

            let text =
                msg_game_started(config.lang, config.min, config.max, new_game.attempts_left);
            bot.send_message(msg.chat.id, text).await?;
            return Ok(());
        }

        // Language command: /lang [chat] <en|it>
        if text.to_lowercase().starts_with("/lang") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            let mut lock = state.write().await;

            // helper to parse language token
            let parse_lang = |s: &str| {
                if s.eq_ignore_ascii_case("it") {
                    Some(Lang::It)
                } else if s.eq_ignore_ascii_case("en") {
                    Some(Lang::En)
                } else {
                    None
                }
            };

            if parts.len() == 1 {
                // show effective language
                bot.send_message(msg.chat.id, msg_lang_current(lang, lang))
                    .await?;
                return Ok(());
            } else if parts.len() == 2 {
                if let Some(new_lang) = parse_lang(parts[1]) {
                    if let Some(user) = msg.from.as_ref() {
                        let key = (msg.chat.id.0, user.id.0);
                        lock.user_langs.insert(key, new_lang);
                        bot.send_message(msg.chat.id, msg_lang_set_user(new_lang))
                            .await?;
                        return Ok(());
                    } else {
                        bot.send_message(msg.chat.id, msg_cannot_start(lang))
                            .await?;
                        return Ok(());
                    }
                }
            } else if parts.len() == 3 && parts[1].eq_ignore_ascii_case("chat") {
                if let Some(new_lang) = parse_lang(parts[2]) {
                    // set chat-level language (affects all users in this chat)
                    lock.chat_langs.insert(msg.chat.id.0, new_lang);
                    bot.send_message(msg.chat.id, msg_lang_set_chat(new_lang))
                        .await?;
                    return Ok(());
                }
            }

            bot.send_message(msg.chat.id, msg_lang_invalid(lang))
                .await?;
            return Ok(());
        }

        // Runtime config helper: show current GAME_MIN/GAME_MAX/GAME_ATTEMPTS
        if text.eq_ignore_ascii_case("/config") {
            bot.send_message(
                msg.chat.id,
                msg_config(config.lang, config.min, config.max, config.attempts),
            )
            .await?;
            return Ok(());
        }

        // If the user does not have a game yet and the message is not a number/command,
        // send a short welcome prompting to use /gioco to start a personal game.
        if let Some(user) = msg.from.as_ref() {
            let key = (msg.chat.id.0, user.id.0);
            let lock_read = state.read().await;
            let has_game = lock_read.by_user.contains_key(&key);
            drop(lock_read);

            if !has_game && !text.starts_with('/') && text.parse::<i32>().is_err() {
                let name = user.first_name.clone();
                bot.send_message(msg.chat.id, msg_welcome_prompt(config.lang, &name))
                    .await?;
                return Ok(());
            }
        }

        // If the message is not a command, try to interpret it as a guess
        if let Ok(guess) = text.parse::<i32>() {
            let mut lock = state.write().await;
            // require user context for per-user games
            let user_id = match msg.from.as_ref().map(|u| u.id.0) {
                Some(id) => id,
                None => {
                    bot.send_message(msg.chat.id, msg_cannot_guess(config.lang))
                        .await?;
                    return Ok(());
                }
            };

            let key = (msg.chat.id.0, user_id);
            if let Some(game) = lock.by_user.get_mut(&key) {
                if game.attempts_left == 0 {
                    bot.send_message(msg.chat.id, msg_no_attempts(config.lang))
                        .await?;
                    return Ok(());
                }

                game.attempts_left = game.attempts_left.saturating_sub(1);

                if guess == game.target {
                    bot.send_message(msg.chat.id, "✅ Esatto! Hai indovinato! Gioco resettato.")
                        .await?;
                    // Reset game for this user using configured range/attempts
                    *game = GameState {
                        target: rand_in_range(config.min, config.max),
                        attempts_left: config.attempts,
                    };
                } else {
                    // If no attempts remain after this wrong guess, reveal the number and
                    // instruct the user to restart with /gioco. Do not auto-reset here.
                    if game.attempts_left == 0 {
                        bot.send_message(msg.chat.id, msg_revealed(config.lang, game.target))
                            .await?;
                    } else if guess < game.target {
                        bot.send_message(msg.chat.id, msg_too_low(config.lang, game.attempts_left))
                            .await?;
                    } else {
                        bot.send_message(
                            msg.chat.id,
                            msg_too_high(config.lang, game.attempts_left),
                        )
                        .await?;
                    }
                }
            } else {
                // No game for this user
                bot.send_message(
                    msg.chat.id,
                    "Non hai ancora una partita attiva. Usa /gioco per iniziare.",
                )
                .await?;
            }
        }
    }

    Ok(())
}

fn rand_in_range(min: i32, max: i32) -> i32 {
    // Use the rand crate's thread-local RNG for uniform sampling
    rand::rng().random_range(min..=max)
}
