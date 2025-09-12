use telegram_bot_rust::next_attempts_after_win;

#[test]
fn consecutive_quick_wins_decrement_start_attempts() {
    let mut start = 10;
    let threshold = 3;
    // simulate three consecutive wins where the user used 1 attempt each time
    for expected in &[9, 8, 7] {
        // remaining_after_guess = start - attempts_used (attempts_used=1)
        let remaining_after_guess = start - 1;
        let next = next_attempts_after_win(start, remaining_after_guess, threshold);
        assert_eq!(next, *expected, "start {} -> next {}", start, next);
        start = next;
    }
}

#[test]
fn any_win_decrements_by_one() {
    let start = 10;
    let threshold = 3;
    // Under the new behavior every win reduces the next start attempts by 1,
    // regardless of how many attempts were used. remaining_after_guess here
    // represents a "slow" win (used 4 attempts).
    let remaining_after_guess = start - 4; // used = 4
    let next = next_attempts_after_win(start, remaining_after_guess, threshold);
    assert_eq!(next, 9);
}
