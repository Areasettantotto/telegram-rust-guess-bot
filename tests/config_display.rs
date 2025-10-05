use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use telegram_bot_rust::*;
use tokio::sync::RwLock;

#[tokio::test]
async fn config_shows_active_attempts_and_remaining_number_attempts() {
    // Build a shared AppState with a user who has an active game and a miss streak of 1.
    let mut by_user = HashMap::new();
    let user_game = GameState {
        target: 42,
        attempts_left: 5,
        start_attempts: 10,
    };
    let chat_id = 123i64;
    let user_id = 456u64;
    by_user.insert((chat_id, user_id), user_game);

    let mut user_miss_streaks = HashMap::new();
    let composite = format!("{}:{}", chat_id, user_id);
    user_miss_streaks.insert(composite.clone(), 1i32);

    let state = Arc::new(RwLock::new(AppState {
        by_user,
        user_langs: HashMap::new(),
        chat_langs: HashMap::new(),
        seen_welcome: HashMap::new(),
        user_start_attempts: HashMap::new(),
        user_miss_streaks,
    }));

    // Build a config with restart_threshold = 3
    let mut messages = HashMap::new();
    // Ensure the config template contains the {number_attempts} placeholder
    let mut msgs = default_messages(Lang::En);
    msgs.config = "Current configuration: min = {min}, max = {max}, attempts = {attempts}, number_attempts = {number_attempts}".to_string();
    messages.insert("it".to_string(), msgs);

    let cfg = Config {
        min: 1,
        max: 100,
        attempts: 10,
        restart_threshold: 3,
        lang: Lang::It,
        messages,
        ttl_seconds: 60 * 60 * 24,
        bot_owner_id: None,
        reset_user_starts: HashSet::new(),
    };
    let shared_cfg = Arc::new(cfg);

    // replicate the /config formatting logic
    let lock = state.read().await;
    let key = (chat_id, user_id);
    let attempts_str = if let Some(game) = lock.by_user.get(&key) {
        game.attempts_left.to_string()
    } else {
        shared_cfg.attempts.to_string()
    };
    let streak = lock.user_miss_streaks.get(&composite).copied().unwrap_or(0);
    let remaining = if streak >= shared_cfg.restart_threshold {
        0
    } else {
        shared_cfg.restart_threshold - streak
    };
    let number_attempts_s = remaining.to_string();
    drop(lock);

    let msgs = &shared_cfg.messages.get("it").expect("it messages");
    let out = format_with(
        &msgs.config,
        &[
            ("min", &shared_cfg.min.to_string()),
            ("max", &shared_cfg.max.to_string()),
            ("attempts", &attempts_str),
            ("number_attempts", &number_attempts_s),
        ],
    );

    assert!(out.contains("attempts = 5"), "output was: {}", out);
    assert!(out.contains("number_attempts = 2"), "output was: {}", out);
}

#[tokio::test]
async fn config_uses_real_italian_messages_file() {
    use std::fs;
    // load messages/it.json from workspace
    let path = std::path::Path::new("messages").join("it.json");
    let s = fs::read_to_string(&path).expect("failed to read messages/it.json");
    let msgs: Messages = serde_json::from_str(&s).expect("failed to parse it.json");

    // Build a minimal state with active game and miss streak = 1
    let mut by_user = HashMap::new();
    let user_game = GameState {
        target: 99,
        attempts_left: 5,
        start_attempts: 10,
    };
    let chat_id = 10i64;
    let user_id = 20u64;
    by_user.insert((chat_id, user_id), user_game);

    let mut user_miss_streaks = HashMap::new();
    let composite = format!("{}:{}", chat_id, user_id);
    user_miss_streaks.insert(composite.clone(), 1i32);

    let state = Arc::new(RwLock::new(AppState {
        by_user,
        user_langs: HashMap::new(),
        chat_langs: HashMap::new(),
        seen_welcome: HashMap::new(),
        user_start_attempts: HashMap::new(),
        user_miss_streaks,
    }));

    let mut messages_map = HashMap::new();
    messages_map.insert("it".to_string(), msgs.clone());

    let cfg = Config {
        min: 1,
        max: 100,
        attempts: 10,
        restart_threshold: 3,
        lang: Lang::It,
        messages: messages_map,
        ttl_seconds: 60 * 60 * 24,
        bot_owner_id: None,
        reset_user_starts: HashSet::new(),
    };
    let shared_cfg = Arc::new(cfg);

    // replicate formatting logic
    let lock = state.read().await;
    let key = (chat_id, user_id);
    let attempts_str = if let Some(game) = lock.by_user.get(&key) {
        game.attempts_left.to_string()
    } else {
        shared_cfg.attempts.to_string()
    };
    let streak = lock.user_miss_streaks.get(&composite).copied().unwrap_or(0);
    let remaining = if streak >= shared_cfg.restart_threshold {
        0
    } else {
        shared_cfg.restart_threshold - streak
    };
    let number_attempts_s = remaining.to_string();
    drop(lock);

    let out = format_with(
        &msgs.config,
        &[
            ("min", &shared_cfg.min.to_string()),
            ("max", &shared_cfg.max.to_string()),
            ("attempts", &attempts_str),
            ("number_attempts", &number_attempts_s),
        ],
    );

    // Check that Italian text is present and numbers substituted
    assert!(
        out.contains("Tentativi per indovinare") || out.contains("attempti"),
        "output: {}",
        out
    );
    assert!(out.contains(&attempts_str), "attempts missing in: {}", out);
    assert!(
        out.contains(&number_attempts_s),
        "number_attempts missing in: {}",
        out
    );
}
