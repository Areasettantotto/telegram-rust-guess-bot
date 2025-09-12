use telegram_bot_rust::{GameState, next_attempts_after_win};

#[test]
fn simulate_win_resets_and_decrements_start_attempts() {
    // initial game started with 10 attempts
    let mut game = GameState {
        target: 42,
        attempts_left: 10,
        start_attempts: 10,
    };
    // simulate a winning guess on the first try: decrement then check
    game.attempts_left = game.attempts_left.saturating_sub(1); // now 9
    assert_eq!(game.attempts_left, 9);
    // compute next attempts using start_attempts
    let next = next_attempts_after_win(game.start_attempts, game.attempts_left, 3);
    assert_eq!(next, 9, "expected start attempts to decrement from 10 to 9");
    // reset the game as the handler would
    game = GameState {
        target: 7,
        attempts_left: next,
        start_attempts: next,
    };
    assert_eq!(game.start_attempts, 9);
}
