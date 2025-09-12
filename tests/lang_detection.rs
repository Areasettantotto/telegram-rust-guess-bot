use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

use telegram_bot_rust::{AppState, Lang, effective_lang_from_parts};

#[test]
fn detects_language_from_message_language_code() {
    let state = Arc::new(tokio::sync::RwLock::new(AppState {
        by_user: HashMap::new(),
        user_langs: HashMap::new(),
        chat_langs: HashMap::new(),
        seen_welcome: HashMap::new(),
        user_start_attempts: HashMap::new(),
        user_miss_streaks: HashMap::new(),
    }));

    let rt = Runtime::new().unwrap();
    let detected = rt.block_on(async {
        effective_lang_from_parts(&state, Some("it"), Some(200), 100, Lang::En).await
    });
    assert_eq!(detected, Lang::It);
}

#[test]
fn detects_language_from_message_language_code_prefix() {
    let state = Arc::new(tokio::sync::RwLock::new(AppState {
        by_user: HashMap::new(),
        user_langs: HashMap::new(),
        chat_langs: HashMap::new(),
        seen_welcome: HashMap::new(),
        user_start_attempts: HashMap::new(),
        user_miss_streaks: HashMap::new(),
    }));

    let rt = Runtime::new().unwrap();
    // simulate a locale-style language_code like "en-US"; the code should accept the prefix
    let detected = rt.block_on(async {
        effective_lang_from_parts(&state, Some("en-US"), Some(300), 101, Lang::It).await
    });
    assert_eq!(detected, Lang::En);
}
