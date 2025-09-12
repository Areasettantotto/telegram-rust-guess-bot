use std::collections::HashMap;
use std::sync::Arc;
use telegram_bot_rust::{AppState, Lang, SharedState, parse_lang, rand_in_range};
use tokio::sync::RwLock;

#[tokio::test]
async fn integration_test_lang_and_rand() {
    // test parse_lang
    assert_eq!(parse_lang("en"), Some(Lang::En));
    assert_eq!(parse_lang("it"), Some(Lang::It));
    assert_eq!(parse_lang("xx"), None);

    // test rand_in_range bounds
    for _ in 0..1_000 {
        let v = rand_in_range(-5, 5);
        assert!(v >= -5 && v <= 5);
    }

    // test updating AppState user_langs directly
    let state: SharedState = Arc::new(RwLock::new(AppState {
        by_user: HashMap::new(),
        user_langs: HashMap::new(),
        chat_langs: HashMap::new(),
        seen_welcome: HashMap::new(),
        user_start_attempts: HashMap::new(),
        user_miss_streaks: HashMap::new(),
    }));

    // insert language
    {
        let mut w = state.write().await;
        w.user_langs.insert((1i64, 1u64), Lang::En);
    }
    {
        let r = state.read().await;
        assert_eq!(r.user_langs.get(&(1i64, 1u64)).cloned(), Some(Lang::En));
    }
}
