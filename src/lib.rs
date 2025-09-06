use anyhow::Result;
use dotenvy::dotenv;
use rand::{Rng, distributions::Uniform};
use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
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
    // users already shown the welcome prompt (per chat) -> unix timestamp
    pub seen_welcome: HashMap<String, u64>,
}

pub type SharedState = Arc<RwLock<AppState>>;

/// Runtime configuration (from environment with sensible defaults)
#[derive(Clone, Debug)]
pub struct Config {
    pub min: i32,
    pub max: i32,
    pub attempts: i32,
    pub lang: Lang,
    pub messages: HashMap<String, Messages>,
    pub ttl_seconds: u64,
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
    pub current_language_label: String,
    pub language_name: String,
    pub success_correct: String,
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
            current_language_label: "Current language:".to_string(),
            language_name: "English".to_string(),
            success_correct: "✅ Correct! You guessed it. Game reset.".to_string(),
        },
    _ => default_messages(Lang::En),
    }
}

/// Load every `*.json` file from the `messages/` directory and return a map
/// from language tag (Lang) to parsed `Messages` value. Files which fail to
/// parse will fall back to defaults for that language.
pub fn load_all_messages(dir: &str) -> HashMap<String, Messages> {
    let mut map = HashMap::new();
    let p = Path::new(dir);
    if let Ok(entries) = p.read_dir() {
        for entry in entries.flatten() {
            if let Ok(fname) = entry.file_name().into_string() {
                if fname.to_lowercase().ends_with(".json") {
                    let stem = fname.trim_end_matches(".json");
                    if let Some(lang) = parse_lang(stem) {
                        let path = format!("{}/{}", dir, fname);
                        let msgs = load_messages_file(&path, lang);
                        map.insert(lang_tag(&lang).to_string(), msgs);
                    } else {
                        tracing::warn!("skipping unknown language file: {}", fname);
                    }
                }
            }
        }
    }
    map
}

pub fn format_with(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut s = template.to_string();
    for (k, v) in pairs {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_seen_welcome(path: &Path) -> HashMap<String, u64> {
    if !path.exists() {
        return HashMap::new();
    }
    match fs::read_to_string(path) {
        Ok(s) => match serde_json::from_str::<HashMap<String, u64>>(&s) {
            Ok(m) => m,
            Err(_) => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    }
}

fn save_seen_welcome(path: &Path, map: &HashMap<String, u64>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(map) {
        let _ = fs::write(path, s);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    It,
    Ar,
    Ru,
    Zh,
}

/// Parse a short language tag into `Lang`.
pub fn parse_lang(s: &str) -> Option<Lang> {
    match s.to_lowercase().as_str() {
        "it" => Some(Lang::It),
        "en" => Some(Lang::En),
        "ar" => Some(Lang::Ar),
        "ru" => Some(Lang::Ru),
        "zh" => Some(Lang::Zh),
        _ => None,
    }
}

/// Return the short tag for a Lang variant (e.g. Lang::En -> "en").
fn lang_tag(l: &Lang) -> &'static str {
    match l {
        Lang::En => "en",
        Lang::It => "it",
        Lang::Ar => "ar",
        Lang::Ru => "ru",
        Lang::Zh => "zh",
    }
}

pub fn rand_in_range(min: i32, max: i32) -> i32 {
    // Use Uniform distribution and a thread-local RNG (non-deprecated API)
    let mut rng = rand::thread_rng();
    let distr = Uniform::new_inclusive(min, max);
    rng.sample(distr)
}

pub async fn effective_lang(state: &SharedState, msg: &Message, default: Lang) -> Lang {
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
    // If the Telegram user provided a language_code, try to respect it for new users
    if let Some(user) = msg.from.as_ref() {
        if let Some(lang_code) = user.language_code.as_ref() {
            // parse_lang expects short tags like "en", "it", etc.
            if let Some(parsed) = parse_lang(lang_code) {
                return parsed;
            }
            // sometimes language_code can be full locale like "en-US"; try prefix
            if lang_code.len() >= 2 {
                let prefix = &lang_code[..2];
                if let Some(parsed) = parse_lang(prefix) {
                    return parsed;
                }
            }
        }
    }

    default
}

/// Test helper: determine effective language given simple parts (used by tests)
pub async fn effective_lang_from_parts(
    state: &SharedState,
    user_language_code: Option<&str>,
    user_id: Option<u64>,
    chat_id: i64,
    default: Lang,
) -> Lang {
    let lock = state.read().await;
    if let Some(uid) = user_id {
        let key = (chat_id, uid);
        if let Some(&l) = lock.user_langs.get(&key) {
            return l;
        }
    }
    if let Some(&l) = lock.chat_langs.get(&chat_id) {
        return l;
    }
    drop(lock);
    if let Some(lang_code) = user_language_code {
        if let Some(parsed) = parse_lang(lang_code) {
            return parsed;
        }
        if lang_code.len() >= 2 {
            let prefix = &lang_code[..2];
            if let Some(parsed) = parse_lang(prefix) {
                return parsed;
            }
        }
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
    // pick messages by language from the map, fall back to config.lang then to English
    // look up messages by the language tag (strings), falling back to config.lang then to English
    let messages = config
        .messages
        .get(lang_tag(&lang))
        .or_else(|| config.messages.get(lang_tag(&config.lang)))
        .or_else(|| config.messages.get("en"))
        .expect("there should always be at least English messages available");

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
                // reply using a localized label and build the current language dynamically
                // keys are language tags (String). We'll build a richer display below.
                // Build the current language display: label + space + "Name (tag)" using the loaded Messages
                // messages for the effective language provide `language_name` and `current_language_label`.
                let current = format!(
                    "{} {} ({})",
                    messages.current_language_label,
                    messages.language_name,
                    lang_tag(&lang)
                );
                let mut reply = current;
                reply.push_str("\n");
                // Build the available languages display: iterate over keys but show name+tag where possible
                let mut available_items: Vec<String> = config
                    .messages
                    .iter()
                    .map(|(k, v)| format!("{} ({})", v.language_name, k))
                    .collect();
                available_items.sort_unstable();
                let available = available_items.join(", ");
                reply.push_str(&format!("Available languages: {}", available));
                bot.send_message(msg.chat.id, reply).await?;
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
                // persisted welcome: key is "chat:user" string
                let composite = format!("{}:{}", msg.chat.id.0, user.id.0);
                let mut lock = state.write().await;
                let seen_ts = lock.seen_welcome.get(&composite).copied().unwrap_or(0);
                let now = now_unix();
                if seen_ts == 0 || now.saturating_sub(seen_ts) > config.ttl_seconds {
                    let name = user.first_name.clone();
                    let reply = format_with(&messages.welcome_prompt, &[("name", name.as_str())]);
                    bot.send_message(msg.chat.id, reply).await?;
                    lock.seen_welcome.insert(composite.clone(), now);
                    // persist to disk; visible path used in run_bot
                    let data_path = Path::new("data").join("seen_welcome.json");
                    let _ = save_seen_welcome(&data_path, &lock.seen_welcome);
                    return Ok(());
                }
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
                    bot.send_message(msg.chat.id, messages.success_correct.clone())
                        .await?;
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

#[cfg(test)]
mod ttl_tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn seen_welcome_ttl_renewal() {
        // temp file path
        let tmp = std::env::temp_dir().join(format!("seen_welcome_test_{}.json", now_unix()));
        let key = "123:456".to_string();
        // create a timestamp older than ttl (120s ago)
        let old_ts = now_unix().saturating_sub(120);
        let mut m = HashMap::new();
        m.insert(key.clone(), old_ts);
        // save and load
        save_seen_welcome(&tmp, &m);
        let loaded = load_seen_welcome(&tmp);
        assert_eq!(loaded.get(&key).copied().unwrap(), old_ts);

        // simulate ttl = 60s -> should be considered expired and renewed
        let ttl = 60u64;
        let now = now_unix();
        if now.saturating_sub(old_ts) > ttl {
            let mut new = loaded.clone();
            new.insert(key.clone(), now);
            save_seen_welcome(&tmp, &new);
        }

        let reloaded = load_seen_welcome(&tmp);
        let renewed_ts = reloaded.get(&key).copied().unwrap();
        // renewed_ts should be >= now
        assert!(renewed_ts >= now);

        let _ = fs::remove_file(&tmp);
    }
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

    let mut cfg = Config {
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
        messages: load_all_messages("messages"),
        ttl_seconds: env::var("SEEN_WELCOME_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60 * 60 * 24 * 30),
    };
    // Ensure at least English messages exist as a fallback
    if !cfg.messages.contains_key("en") {
        cfg.messages.insert(
            "en".to_string(),
            load_messages_file("messages/en.json", Lang::En),
        );
    }
    // Ensure default language exists in the map; if not, insert fallback
    let cfg_lang_tag = lang_tag(&cfg.lang).to_string();
    if !cfg.messages.contains_key(&cfg_lang_tag) {
        cfg.messages.insert(
            cfg_lang_tag.clone(),
            cfg.messages.get("en").unwrap().clone(),
        );
    }
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

    // load persisted seen_welcome map
    let data_dir = Path::new("data");
    let seen_path = data_dir.join("seen_welcome.json");
    let seen_welcome = load_seen_welcome(&seen_path);

    let state = Arc::new(RwLock::new(AppState {
        by_user: HashMap::new(),
        user_langs: HashMap::new(),
        chat_langs: HashMap::new(),
        seen_welcome,
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
