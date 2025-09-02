use anyhow::Result;
use dotenvy::dotenv;
use rand::{Rng, distributions::Uniform};
use serde::Deserialize;
use std::{collections::HashMap, env, fs, sync::Arc};
use teloxide::prelude::*;
use tokio::sync::RwLock;

/// State of a single game for a user in a chat
#[derive(Clone, Debug)]
pub struct GameState {
    pub target: i32,
    pub attempts_left: i32,
}

/// Shared application state
pub struct AppState {
    // key: (chat_id, user_id)
    pub by_user: HashMap<(i64, u64), GameState>,
    // language preferences
    pub user_langs: HashMap<(i64, u64), Lang>,
    pub chat_langs: HashMap<i64, Lang>,
}

pub type SharedState = Arc<RwLock<AppState>>;

/// Runtime configuration (from environment with sensible defaults)
#[derive(Clone, Debug)]
pub struct Config {
    pub min: i32,
    pub max: i32,
    pub attempts: i32,
    pub lang: Lang,
    pub messages_en: Messages,
    pub messages_it: Messages,
}

pub type SharedConfig = Arc<Config>;

/// Messages container loaded from JSON files per language
#[derive(Clone, Debug, Deserialize)]
pub struct Messages {
    pub cannot_start: String,
    pub cannot_guess: String,
    pub game_started: String,
    pub config: String,
    pub welcome_prompt: String,
    pub no_attempts: String,
    pub revealed: String,
    pub too_low: String,
    pub too_high: String,
    pub lang_set_user: String,
    pub lang_set_chat: String,
    pub lang_invalid: String,
    pub pong: String,
    pub not_started_prompt: String,
}

pub fn load_messages_file(path: &str, lang: Lang) -> Messages {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            tracing::warn!("failed to parse {}: {}. Falling back to defaults.", path, e);
            default_messages(lang)
        }),
        Err(e) => {
            tracing::warn!("failed to read {}: {}. Falling back to defaults.", path, e);
            default_messages(lang)
        }
    }
}

pub fn default_messages(lang: Lang) -> Messages {
    match lang {
        Lang::En => Messages {
            cannot_start: "I can't start a game for channels or messages without a user.".to_string(),
            cannot_guess: "I can't handle guesses without a user.".to_string(),
            game_started: "🎯 Game started for you! Guess a number between {min} and {max}. Attempts left: {attempts}\n".to_string(),
            config: "Current configuration: min = {min}, max = {max}, attempts = {attempts}".to_string(),
            welcome_prompt: "Hi {name}! Use /gioco to start your personal game.".to_string(),
            no_attempts: "No attempts left. Use /gioco to restart.".to_string(),
            revealed: "❌ You've run out of attempts. The number was {target}. Use /gioco to restart.".to_string(),
            too_low: "Too low. Attempts left: {attempts}".to_string(),
            too_high: "Too high. Attempts left: {attempts}".to_string(),
            lang_set_user: "Your language preference was set.".to_string(),
            lang_set_chat: "Chat language preference was set.".to_string(),
            lang_invalid: "Invalid usage. Examples: `/lang en`, `/lang it`, `/lang chat en`".to_string(),
            pong: "pong".to_string(),
            not_started_prompt: "You don't have an active game yet. Use /gioco to start.".to_string(),
        },
        Lang::It => Messages {
            cannot_start: "Non posso avviare una partita per canali o messaggi senza utente.".to_string(),
            cannot_guess: "Non posso gestire congetture senza un utente.".to_string(),
            game_started: "🎯 Gioco avviato per te! Indovina un numero tra {min} e {max}. Tentativi rimasti: {attempts}\n".to_string(),
            config: "Configurazione corrente: min = {min}, max = {max}, tentativi = {attempts}".to_string(),
            welcome_prompt: "Ciao {name}! Usa /gioco per iniziare la tua partita personale.".to_string(),
            no_attempts: "Nessun tentativo rimasto. Usa /gioco per ricominciare.".to_string(),
            revealed: "❌ Hai esaurito i tentativi. Il numero era {target}. Usa /gioco per ricominciare.".to_string(),
            too_low: "Troppo basso. Tentativi rimasti: {attempts}".to_string(),
            too_high: "Troppo alto. Tentativi rimasti: {attempts}".to_string(),
            lang_set_user: "La tua lingua è stata impostata.".to_string(),
            lang_set_chat: "La lingua della chat è stata impostata.".to_string(),
            lang_invalid: "Uso non valido. Esempi: `/lang en`, `/lang it`, `/lang chat en`".to_string(),
            pong: "pong".to_string(),
            not_started_prompt: "Non hai ancora una partita attiva. Usa /gioco per iniziare.".to_string(),
        },
    }
}

pub fn format_with(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut s = template.to_string();
    for (k, v) in pairs {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    It,
}

/// Parse a short language tag into `Lang`.
pub fn parse_lang(s: &str) -> Option<Lang> {
    if s.eq_ignore_ascii_case("it") {
        Some(Lang::It)
    } else if s.eq_ignore_ascii_case("en") {
        Some(Lang::En)
    } else {
        None
    }
}

pub fn rand_in_range(min: i32, max: i32) -> i32 {
    // Use Uniform distribution and a thread-local RNG (non-deprecated API)
    let mut rng = rand::thread_rng();
    let distr = Uniform::new_inclusive(min, max);
    rng.sample(distr)
}

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

async fn handle_message(
    bot: &Bot,
    msg: &Message,
    state: SharedState,
    config: SharedConfig,
) -> Result<()> {
    let lang = effective_lang(&state, msg, config.lang).await;
    let messages = match lang {
        Lang::En => &config.messages_en,
        Lang::It => &config.messages_it,
    };

    if let Some(text) = msg.text() {
        let text = text.trim();
        if text.eq_ignore_ascii_case("/ping") {
            bot.send_message(msg.chat.id, messages.pong.clone()).await?;
            return Ok(());
        }

        if text.eq_ignore_ascii_case("/gioco") {
            let user_id = match msg.from.as_ref().map(|u| u.id.0) {
                Some(id) => id,
                None => {
                    bot.send_message(msg.chat.id, messages.cannot_start.clone())
                        .await?;
                    return Ok(());
                }
            };
            let mut lock = state.write().await;
            let key = (msg.chat.id.0, user_id);
            let new_game = GameState {
                target: rand_in_range(config.min, config.max),
                attempts_left: config.attempts,
            };
            lock.by_user.insert(key, new_game.clone());
            let reply = format_with(
                &messages.game_started,
                &[
                    ("min", &config.min.to_string()),
                    ("max", &config.max.to_string()),
                    ("attempts", &new_game.attempts_left.to_string()),
                ],
            );
            bot.send_message(msg.chat.id, reply).await?;
            return Ok(());
        }

        if text.to_lowercase().starts_with("/lang") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            let mut lock = state.write().await;
            if parts.len() == 1 {
                let current = match lang {
                    Lang::En => "Current language: English (en)",
                    Lang::It => "Lingua corrente: Italiano (it)",
                };
                bot.send_message(msg.chat.id, current).await?;
                return Ok(());
            }
            if parts.len() == 2 {
                if let Some(new_lang) = crate::parse_lang(parts[1]) {
                    if let Some(user) = msg.from.as_ref() {
                        let key = (msg.chat.id.0, user.id.0);
                        lock.user_langs.insert(key, new_lang);
                        bot.send_message(msg.chat.id, messages.lang_set_user.clone())
                            .await?;
                        return Ok(());
                    } else {
                        bot.send_message(msg.chat.id, messages.cannot_start.clone())
                            .await?;
                        return Ok(());
                    }
                }
            }
            if parts.len() == 3 && parts[1].eq_ignore_ascii_case("chat") {
                if let Some(new_lang) = crate::parse_lang(parts[2]) {
                    lock.chat_langs.insert(msg.chat.id.0, new_lang);
                    bot.send_message(msg.chat.id, messages.lang_set_chat.clone())
                        .await?;
                    return Ok(());
                }
            }
            bot.send_message(msg.chat.id, messages.lang_invalid.clone())
                .await?;
            return Ok(());
        }

        if text.eq_ignore_ascii_case("/config") {
            let reply = format_with(
                &messages.config,
                &[
                    ("min", &config.min.to_string()),
                    ("max", &config.max.to_string()),
                    ("attempts", &config.attempts.to_string()),
                ],
            );
            bot.send_message(msg.chat.id, reply).await?;
            return Ok(());
        }

        if let Some(user) = msg.from.as_ref() {
            let key = (msg.chat.id.0, user.id.0);
            let lock_read = state.read().await;
            let has_game = lock_read.by_user.contains_key(&key);
            drop(lock_read);
            if !has_game && !text.starts_with('/') && text.parse::<i32>().is_err() {
                let name = user.first_name.clone();
                let reply = format_with(&messages.welcome_prompt, &[("name", &name)]);
                bot.send_message(msg.chat.id, reply).await?;
                return Ok(());
            }
        }

        if let Ok(guess) = text.parse::<i32>() {
            let mut lock = state.write().await;
            let user_id = match msg.from.as_ref().map(|u| u.id.0) {
                Some(id) => id,
                None => {
                    bot.send_message(msg.chat.id, messages.cannot_guess.clone())
                        .await?;
                    return Ok(());
                }
            };
            let key = (msg.chat.id.0, user_id);
            if let Some(game) = lock.by_user.get_mut(&key) {
                if game.attempts_left == 0 {
                    bot.send_message(msg.chat.id, messages.no_attempts.clone())
                        .await?;
                    return Ok(());
                }
                game.attempts_left = game.attempts_left.saturating_sub(1);
                if guess == game.target {
                    let success = match lang {
                        Lang::En => "✅ Correct! You guessed it. Game reset.".to_string(),
                        Lang::It => "✅ Esatto! Hai indovinato! Gioco resettato.".to_string(),
                    };
                    bot.send_message(msg.chat.id, success).await?;
                    *game = GameState {
                        target: rand_in_range(config.min, config.max),
                        attempts_left: config.attempts,
                    };
                } else {
                    if game.attempts_left == 0 {
                        let reply = format_with(
                            &messages.revealed,
                            &[("target", &game.target.to_string())],
                        );
                        bot.send_message(msg.chat.id, reply).await?;
                    } else if guess < game.target {
                        let reply = format_with(
                            &messages.too_low,
                            &[("attempts", &game.attempts_left.to_string())],
                        );
                        bot.send_message(msg.chat.id, reply).await?;
                    } else {
                        let reply = format_with(
                            &messages.too_high,
                            &[("attempts", &game.attempts_left.to_string())],
                        );
                        bot.send_message(msg.chat.id, reply).await?;
                    }
                }
            } else {
                bot.send_message(msg.chat.id, messages.not_started_prompt.clone())
                    .await?;
            }
        }
    }

    Ok(())
}

/// Run the bot (previously in main). Separated so binaries can call this and
/// tests/integration can import the library.
pub async fn run_bot() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().ok();
    let bot = Bot::from_env();

    let default_lang = env::var("DEFAULT_LANG")
        .ok()
        .unwrap_or_else(|| "en".to_string());
    let default_lang = if default_lang.to_lowercase().starts_with("it") {
        Lang::It
    } else {
        Lang::En
    };

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
        lang: default_lang,
        messages_en: load_messages_file("messages/en.json", Lang::En),
        messages_it: load_messages_file("messages/it.json", Lang::It),
    };
    let shared_config = Arc::new(cfg);

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
