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
                    if let Some(_lang) = parse_lang(stem) {
                        expected.push(stem.to_string());
                    }
                }
            }
        }
    }

    // load via library
    let map = load_all_messages("messages");

    for lang in expected {
        assert!(
            map.contains_key(&lang),
            "messages map missing language: {}",
            lang
        );
    }

    // ensure each loaded Messages has a non-empty current_language_label and language_name
    for (k, v) in map.iter() {
        assert!(
            !v.current_language_label.trim().is_empty(),
            "{} missing current_language_label",
            k
        );
        assert!(
            !v.language_name.trim().is_empty(),
            "{} missing language_name",
            k
        );
    }
}
