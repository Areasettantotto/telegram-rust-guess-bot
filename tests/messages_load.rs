use std::fs;
use std::path::Path;
use telegram_bot_rust::{load_all_messages, parse_lang};

#[test]
fn messages_dir_loads_all_known_languages() {
    // list files under messages/
    let dir = Path::new("messages");
    let mut expected = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            if let Some(fname) = e.file_name().to_str() {
                if fname.to_lowercase().ends_with(".json") {
                    let stem = fname.trim_end_matches(".json");
                    if let Some(lang) = parse_lang(stem) {
                        expected.push(lang);
                    }
                }
            }
        }
    }

    // load via library
    let map = load_all_messages("messages");

    for lang in expected {
        assert!(map.contains_key(&lang), "messages map missing language: {:?}", lang);
    }
}
