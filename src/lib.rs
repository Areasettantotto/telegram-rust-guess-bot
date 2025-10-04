use anyhow::Result;
use dotenvy::dotenv;
use rand::{Rng, distributions::Uniform};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
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
    /// how many attempts this game started with (used to compute next game's
    /// starting attempts when a quick win occurs)
    pub start_attempts: i32,
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
    // persisted map of "chat:user" -> start_attempts for the next/new game
    pub user_start_attempts: HashMap<String, i32>,
    // persisted map of "chat:user" -> consecutive non-quick-win losses
    pub user_miss_streaks: HashMap<String, i32>,
}

pub type SharedState = Arc<RwLock<AppState>>;

/// Runtime configuration (from environment with sensible defaults)
#[derive(Clone, Debug)]
pub struct Config {
    pub min: i32,
    pub max: i32,
    pub attempts: i32,
    // If a user guesses the number within this many attempts, the next
    // game for that user will start with attempts reduced by 1.
    // Environment variable: NUMBER_ATTEMPTS
    pub restart_threshold: i32,
    pub lang: Lang,
    pub messages: HashMap<String, Messages>,
    pub ttl_seconds: u64,
    pub bot_owner_id: Option<u64>,
    // set of "chat:user" strings allowed to call /reset_starts (from RESET_USER_STARTS)
    pub reset_user_starts: HashSet<String>,
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
    pub reset_starts_ok: String,
    pub success_correct: String,
}

/// Load a Messages struct from a given JSON file path, falling back to defaults
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

/// Return default Messages for a given language; currently only English is supported.
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
            reset_starts_ok: "Persisted per-user start attempts cleared.".to_string(),
            success_correct: "✅ You guessed it!! Guess a new random number in {next_attempts} attempts. You will have {number_attempts} attempts before failing and starting over.".to_string(),
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

/// Simple template formatter: replace `{key}` with `value` for each pair in `pairs`.
pub fn format_with(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut s = template.to_string();
    for (k, v) in pairs {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

/// Return the current unix timestamp in seconds
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Persisted seen welcome map helpers
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

/// Save the seen_welcome map to the given path as pretty JSON
fn save_seen_welcome(path: &Path, map: &HashMap<String, u64>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(map) {
        let _ = fs::write(path, s);
    }
}

/// Persisted per-user start attempts map helpers
fn load_user_start_attempts(path: &Path) -> HashMap<String, i32> {
    if !path.exists() {
        return HashMap::new();
    }
    match fs::read_to_string(path) {
        Ok(s) => match serde_json::from_str::<HashMap<String, i32>>(&s) {
            Ok(m) => m,
            Err(_) => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    }
}

/// Save the user_start_attempts map to the given path as pretty JSON
fn save_user_start_attempts(path: &Path, map: &HashMap<String, i32>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(map) {
        let _ = fs::write(path, s);
    }
}

/// Persisted per-user miss streaks helpers
fn load_user_miss_streaks(path: &Path) -> HashMap<String, i32> {
    if !path.exists() {
        return HashMap::new();
    }
    match fs::read_to_string(path) {
        Ok(s) => match serde_json::from_str::<HashMap<String, i32>>(&s) {
            Ok(m) => m,
            Err(_) => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    }
}

/// Save the user_miss_streaks map to the given path as pretty JSON
fn save_user_miss_streaks(path: &Path, map: &HashMap<String, i32>) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(map) {
        let _ = fs::write(path, s);
    }
}

/// Supported languages
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

/// Return a random integer in the inclusive range [min, max].
pub fn rand_in_range(min: i32, max: i32) -> i32 {
    // Use Uniform distribution and a thread-local RNG (non-deprecated API)
    let mut rng = rand::thread_rng();
    let distr = Uniform::new_inclusive(min, max);
    rng.sample(distr)
}

/// Compute how many attempts the next game should have after a successful
/// guess. New behavior: every win reduces the starting attempts for the next
/// game by one (to make subsequent wins harder), never going below 1.
/// The `restart_threshold` is still used elsewhere for reset-on-misses logic
/// but is not involved here.
pub fn next_attempts_after_win(
    previous_start_attempts: i32,
    _remaining_after_guess: i32,
    _restart_threshold: i32,
) -> i32 {
    std::cmp::max(1, previous_start_attempts - 1)
}

/// Determine the effective language for a message, given the shared state
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

/// Handle an incoming message, updating state as needed and sending replies.
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
        if text.eq_ignore_ascii_case("/reset_starts") {
            // only allow if BOT_OWNER_ID matches the sender (if configured) or
            // if the composite "chat:user" is present in RESET_USER_STARTS
            let allowed = if let Some(user) = msg.from.as_ref() {
                let is_owner = config.bot_owner_id.map(|o| user.id.0 == o).unwrap_or(false);
                let composite = format!("{}:{}", msg.chat.id.0, user.id.0);
                let in_reset_list = config.reset_user_starts.contains(&composite);
                is_owner || in_reset_list
            } else {
                false
            };
            if !allowed {
                // silently ignore or respond with not permitted; choose to respond
                bot.send_message(msg.chat.id, "Not authorized.").await?;
                return Ok(());
            }
            let mut lock = state.write().await;
            lock.user_start_attempts.clear();
            let data_path = Path::new("data").join("user_start_attempts.json");
            let _ = save_user_start_attempts(&data_path, &lock.user_start_attempts);
            bot.send_message(msg.chat.id, messages.reset_starts_ok.clone())
                .await?;
            return Ok(());
        }
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
            // consult persisted per-user start attempts (if any)
            let composite = format!("{}:{}", msg.chat.id.0, user_id);
            let start_attempts = lock
                .user_start_attempts
                .get(&composite)
                .copied()
                .unwrap_or(config.attempts);
            let new_game = GameState {
                target: rand_in_range(config.min, config.max),
                attempts_left: start_attempts,
                start_attempts,
            };
            lock.by_user.insert(key, new_game.clone());
            // persist the chosen start_attempts for this user so future games (and restarts)
            // will use the same starting value until changed by a win
            lock.user_start_attempts
                .insert(composite.clone(), start_attempts);
            let data_path = Path::new("data").join("user_start_attempts.json");
            let _ = save_user_start_attempts(&data_path, &lock.user_start_attempts);
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
            if let Some(mut game) = lock.by_user.remove(&key) {
                // check if no attempts left before decrement
                if game.attempts_left == 0 {
                    // reinsert and notify
                    lock.by_user.insert(key, game);
                    bot.send_message(msg.chat.id, messages.no_attempts.clone())
                        .await?;
                    return Ok(());
                }

                // decrement owned game attempts
                game.attempts_left = game.attempts_left.saturating_sub(1);

                if guess == game.target {
                    // compute attempts for the next game depending on how fast
                    // the user succeeded. Note: `game.attempts_left` has
                    // already been decremented above, so pass that as
                    // `remaining_after_guess`.
                    let next_attempts = next_attempts_after_win(
                        game.start_attempts,
                        game.attempts_left,
                        config.restart_threshold,
                    );
                    tracing::info!(
                        "win: chat={} user={} prev_start={} remaining_after_guess={} next= {}",
                        msg.chat.id.0,
                        user_id,
                        game.start_attempts,
                        game.attempts_left,
                        next_attempts
                    );
                    // format success message with next_attempts and number_attempts placeholders
                    // Fill placeholders correctly:
                    // - `{next_attempts}` = computed next_attempts (e.g. GAME_ATTEMPTS - 1)
                    // - `{number_attempts}` = restart threshold (NUMBER_ATTEMPTS)
                    let success_msg = format_with(
                        &messages.success_correct,
                        &[
                            ("next_attempts", &next_attempts.to_string()),
                            ("number_attempts", &config.restart_threshold.to_string()),
                        ],
                    );
                    bot.send_message(msg.chat.id, success_msg).await?;

                    // reset game to next_attempts
                    game = GameState {
                        target: rand_in_range(config.min, config.max),
                        attempts_left: next_attempts,
                        start_attempts: next_attempts,
                    };

                    // reset miss streak on a win and persist
                    let composite = format!("{}:{}", msg.chat.id.0, user_id);
                    lock.user_miss_streaks.insert(composite.clone(), 0);
                    lock.user_start_attempts
                        .insert(composite.clone(), next_attempts);
                    // clone maps for persisting
                    let starts_clone = lock.user_start_attempts.clone();
                    let misses_clone = lock.user_miss_streaks.clone();
                    // reinsert updated game then drop lock to persist
                    lock.by_user.insert(key, game);
                    drop(lock);
                    let data_path = Path::new("data").join("user_start_attempts.json");
                    let _ = save_user_start_attempts(&data_path, &starts_clone);
                    let miss_path = Path::new("data").join("user_miss_streaks.json");
                    let _ = save_user_miss_streaks(&miss_path, &misses_clone);
                } else {
                    if game.attempts_left == 0 {
                        // user ran out of attempts -> increment miss streak and persist
                        let composite = format!("{}:{}", msg.chat.id.0, user_id);
                        let streak =
                            lock.user_miss_streaks.get(&composite).copied().unwrap_or(0) + 1;
                        lock.user_miss_streaks.insert(composite.clone(), streak);
                        if streak >= config.restart_threshold {
                            lock.user_start_attempts
                                .insert(composite.clone(), config.attempts);
                            lock.user_miss_streaks.insert(composite.clone(), 0);
                        }
                        let starts_clone = lock.user_start_attempts.clone();
                        let misses_clone = lock.user_miss_streaks.clone();
                        // capture target before moving game back into the map
                        let revealed_target = game.target;
                        // reinsert game (with attempts_left == 0)
                        lock.by_user.insert(key, game);
                        drop(lock);
                        let data_path = Path::new("data").join("user_start_attempts.json");
                        let _ = save_user_start_attempts(&data_path, &starts_clone);
                        let miss_path = Path::new("data").join("user_miss_streaks.json");
                        let _ = save_user_miss_streaks(&miss_path, &misses_clone);

                        let remaining_before_reset = if streak >= config.restart_threshold {
                            0
                        } else {
                            config.restart_threshold - streak
                        };
                        let reply = format_with(
                            &messages.revealed,
                            &[
                                ("target", &revealed_target.to_string()),
                                ("number_attempts", &remaining_before_reset.to_string()),
                            ],
                        );
                        bot.send_message(msg.chat.id, reply).await?;
                    } else if guess < game.target {
                        let reply = format_with(
                            &messages.too_low,
                            &[("attempts", &game.attempts_left.to_string())],
                        );
                        // reinsert game
                        lock.by_user.insert(key, game);
                        bot.send_message(msg.chat.id, reply).await?;
                    } else {
                        let reply = format_with(
                            &messages.too_high,
                            &[("attempts", &game.attempts_left.to_string())],
                        );
                        // reinsert game
                        lock.by_user.insert(key, game);
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

/// Main bot runner: load config, state, and start polling
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

    #[test]
    fn simulate_start_quickwin_restart_cycle() {
        use std::fs;
        // temp file path
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp =
            std::env::temp_dir().join(format!("user_start_attempts_test_{}_{}.json", pid, nanos));
        let composite = "123:456".to_string();
        // ensure clean state
        let _ = fs::remove_file(&tmp);

        // config defaults for the test
        let config_attempts = 10i32;
        let restart_threshold = 3i32;

        // Step 1: no persisted value -> start uses config_attempts
        let loaded = load_user_start_attempts(&tmp);
        assert!(loaded.get(&composite).is_none());
        let start_attempts = loaded.get(&composite).copied().unwrap_or(config_attempts);
        assert_eq!(start_attempts, config_attempts);

        // Step 2: simulate /gioco and persist the start attempts
        let mut map = loaded.clone();
        map.insert(composite.clone(), start_attempts);
        save_user_start_attempts(&tmp, &map);
        let reloaded = load_user_start_attempts(&tmp);
        assert_eq!(reloaded.get(&composite).copied().unwrap(), config_attempts);

        // Step 3: simulate a quick win (used 1 attempt)
        let previous_start = config_attempts;
        let remaining_after_guess = previous_start - 1; // e.g., guessed on first try
        let next =
            next_attempts_after_win(previous_start, remaining_after_guess, restart_threshold);
        assert_eq!(next, previous_start - 1);
        // persist the new start attempts
        let mut updated = reloaded.clone();
        updated.insert(composite.clone(), next);
        save_user_start_attempts(&tmp, &updated);

        // Step 4: simulate restart -> load persisted map and ensure start_attempts is decremented
        let final_map = load_user_start_attempts(&tmp);
        assert_eq!(
            final_map.get(&composite).copied().unwrap(),
            previous_start - 1
        );

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn simulate_consecutive_losses_reset() {
        use std::fs;
        // temp file path for both maps
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp_starts =
            std::env::temp_dir().join(format!("user_start_attempts_test_{}_{}.json", pid, nanos));
        let tmp_miss =
            std::env::temp_dir().join(format!("user_miss_streaks_test_{}_{}.json", pid, nanos));
        let composite = "123:456".to_string();
        let _ = fs::remove_file(&tmp_starts);
        let _ = fs::remove_file(&tmp_miss);

        let config_attempts = 10i32;
        let restart_threshold = 3i32;

        // start with a decremented value (e.g., 9)
        let mut starts = HashMap::new();
        starts.insert(composite.clone(), config_attempts - 1);
        save_user_start_attempts(&tmp_starts, &starts);
        let mut misses = HashMap::new();
        misses.insert(composite.clone(), 0);
        save_user_miss_streaks(&tmp_miss, &misses);

        // simulate three consecutive runs where user fails to guess within threshold
        let mut loaded_starts = load_user_start_attempts(&tmp_starts);
        let mut loaded_misses = load_user_miss_streaks(&tmp_miss);
        for _ in 0..restart_threshold {
            // user runs out -> increment miss
            let streak = loaded_misses.get(&composite).copied().unwrap_or(0) + 1;
            loaded_misses.insert(composite.clone(), streak);
            if streak >= restart_threshold {
                loaded_starts.insert(composite.clone(), config_attempts);
                loaded_misses.insert(composite.clone(), 0);
            }
        }
        // persist and reload
        save_user_start_attempts(&tmp_starts, &loaded_starts);
        save_user_miss_streaks(&tmp_miss, &loaded_misses);
        let final_starts = load_user_start_attempts(&tmp_starts);
        assert_eq!(
            final_starts.get(&composite).copied().unwrap(),
            config_attempts
        );

        let _ = fs::remove_file(&tmp_starts);
        let _ = fs::remove_file(&tmp_miss);
    }

    #[test]
    fn simulate_two_quick_wins_decrement_twice() {
        use std::fs;
        // temp file path
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp =
            std::env::temp_dir().join(format!("user_start_attempts_test_{}_{}.json", pid, nanos));
        let composite = "123:456".to_string();
        let _ = fs::remove_file(&tmp);

        let config_attempts = 10i32;
        let restart_threshold = 3i32;

        // Step 1: initial save as full attempts
        let mut map = HashMap::new();
        map.insert(composite.clone(), config_attempts);
        save_user_start_attempts(&tmp, &map);

        // Simulate first quick win: used 1 attempt -> next should be 9
        let previous = config_attempts;
        let remaining_after_guess = previous - 1;
        let next1 = next_attempts_after_win(previous, remaining_after_guess, restart_threshold);
        assert_eq!(next1, previous - 1);
        map.insert(composite.clone(), next1);
        save_user_start_attempts(&tmp, &map);

        // Simulate second quick win: used 1 attempt on 9 -> next should be 8
        let previous2 = next1;
        let remaining_after_guess2 = previous2 - 1;
        let next2 = next_attempts_after_win(previous2, remaining_after_guess2, restart_threshold);
        assert_eq!(next2, previous2 - 1);
        map.insert(composite.clone(), next2);
        save_user_start_attempts(&tmp, &map);

        // Reload and assert final value is 8
        let final_map = load_user_start_attempts(&tmp);
        assert_eq!(
            final_map.get(&composite).copied().unwrap(),
            config_attempts - 2
        );

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
        restart_threshold: env::var("NUMBER_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
        lang: default_lang,
        messages: load_all_messages("messages"),
        ttl_seconds: env::var("SEEN_WELCOME_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60 * 60 * 24 * 30),
        bot_owner_id: env::var("BOT_OWNER_ID").ok().and_then(|v| v.parse().ok()),
        reset_user_starts: env::var("RESET_USER_STARTS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().trim_matches('"').to_string())
                    .filter(|p| !p.is_empty())
                    .collect::<HashSet<String>>()
            })
            .unwrap_or_default(),
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
    if shared_config.restart_threshold < 0 {
        anyhow::bail!(
            "Invalid configuration: NUMBER_ATTEMPTS ({}) must be a non-negative integer.",
            shared_config.restart_threshold
        );
    }

    // load persisted seen_welcome map
    let data_dir = Path::new("data");
    let seen_path = data_dir.join("seen_welcome.json");
    let seen_welcome = load_seen_welcome(&seen_path);

    // load persisted per-user start attempts map
    let user_start_path = data_dir.join("user_start_attempts.json");
    let user_start_attempts = load_user_start_attempts(&user_start_path);
    // load persisted per-user miss streaks
    let user_miss_path = data_dir.join("user_miss_streaks.json");
    let user_miss_streaks = load_user_miss_streaks(&user_miss_path);

    let state = Arc::new(RwLock::new(AppState {
        by_user: HashMap::new(),
        user_langs: HashMap::new(),
        chat_langs: HashMap::new(),
        seen_welcome,
        user_start_attempts,
        user_miss_streaks,
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
