use telegram_bot_rust::load_all_messages;

#[test]
fn all_languages_have_required_keys() {
    let map = load_all_messages("messages");
    assert!(
        !map.is_empty(),
        "no messages loaded from messages/ directory"
    );

    for (tag, msgs) in map.iter() {
        assert!(
            !msgs.cannot_start.trim().is_empty(),
            "{} missing cannot_start",
            tag
        );
        assert!(
            !msgs.cannot_guess.trim().is_empty(),
            "{} missing cannot_guess",
            tag
        );
        assert!(
            !msgs.game_started.trim().is_empty(),
            "{} missing game_started",
            tag
        );
        assert!(!msgs.config.trim().is_empty(), "{} missing config", tag);
        assert!(
            !msgs.welcome_prompt.trim().is_empty(),
            "{} missing welcome_prompt",
            tag
        );
        assert!(
            !msgs.no_attempts.trim().is_empty(),
            "{} missing no_attempts",
            tag
        );
        assert!(!msgs.revealed.trim().is_empty(), "{} missing revealed", tag);
        assert!(!msgs.too_low.trim().is_empty(), "{} missing too_low", tag);
        assert!(!msgs.too_high.trim().is_empty(), "{} missing too_high", tag);
        assert!(
            !msgs.lang_set_user.trim().is_empty(),
            "{} missing lang_set_user",
            tag
        );
        assert!(
            !msgs.lang_set_chat.trim().is_empty(),
            "{} missing lang_set_chat",
            tag
        );
        assert!(
            !msgs.lang_invalid.trim().is_empty(),
            "{} missing lang_invalid",
            tag
        );
        assert!(!msgs.pong.trim().is_empty(), "{} missing pong", tag);
        assert!(
            !msgs.not_started_prompt.trim().is_empty(),
            "{} missing not_started_prompt",
            tag
        );
        assert!(
            !msgs.current_language_label.trim().is_empty(),
            "{} missing current_language_label",
            tag
        );
        assert!(
            !msgs.language_name.trim().is_empty(),
            "{} missing language_name",
            tag
        );
        assert!(
            !msgs.success_correct.trim().is_empty(),
            "{} missing success_correct",
            tag
        );
    }
}
