use telegram_bot_rust::*;

// This test exercises the core logic for computing next attempts after a win
// without spinning up the full bot. It uses the helper `next_attempts_after_win`.
#[test]
fn reduces_attempts_when_quick_win() {
    // config: 5 attempts, threshold 3 -> if user wins within 3 attempts, next = 4
    let config_attempts = 5;
    let restart_threshold = 3;

    // Under new behavior any win reduces start attempts by 1 regardless of speed.
    // simulate user used 2 attempts (so remaining_after_guess = 3)
    let remaining_after_guess = 3;
    let next = next_attempts_after_win(config_attempts, remaining_after_guess, restart_threshold);
    assert_eq!(next, 4);

    // simulate user used 4 attempts (remaining = 1) -> still decrements by 1
    let remaining_after_guess = 1; // used = 4
    let next2 = next_attempts_after_win(config_attempts, remaining_after_guess, restart_threshold);
    assert_eq!(next2, 4);

    // threshold 0 no longer affects decrement behavior in this helper
    let next3 = next_attempts_after_win(3, 2, 0); // used = 1
    assert_eq!(next3, 2);
}
