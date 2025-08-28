use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use dotenvy::dotenv;
use rand::Rng;
use teloxide::prelude::*;
use tokio::sync::RwLock;

/// Stato della singola partita per una chat
#[derive(Clone, Debug)]
struct GameState {
    target: i32,
    attempts_left: i32,
}

/// Stato applicazione condiviso
struct AppState {
    by_chat: HashMap<i64, GameState>,
}

type SharedState = Arc<RwLock<AppState>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load `.env` file if present so TELOXIDE_TOKEN can be set there.
    dotenv().ok();

    let bot = Bot::from_env();
    let state = Arc::new(RwLock::new(AppState {
        by_chat: HashMap::new(),
    }));

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let state = state.clone();
        async move {
            if let Err(err) = handle_message(&bot, &msg, state).await {
                tracing::error!("handler error: {:?}", err);
            }
            respond(())
        }
    })
    .await;

    Ok(())
}

async fn handle_message(bot: &Bot, msg: &Message, state: SharedState) -> Result<()> {
    if let Some(text) = msg.text() {
        let text = text.trim();

        // Comandi semplici
        if text.eq_ignore_ascii_case("/ping") {
            bot.send_message(msg.chat.id, "pong").await?;
            return Ok(());
        }

        if text.eq_ignore_ascii_case("/gioco") {
            let mut lock = state.write().await;
            let chat_id = msg.chat.id.0;
            let entry = lock.by_chat.entry(chat_id).or_insert_with(|| GameState {
                target: rand_in_range(1, 100),
                attempts_left: 5,
            });

            let text = format!(
                "🎯 Gioco attivo! Indovina un numero tra 1 e 100. Tentativi rimasti: {}\n",
                entry.attempts_left
            );
            bot.send_message(msg.chat.id, text).await?;
            return Ok(());
        }

        // Se messaggio non è comando, prova a interpretarlo come tentativo
        if let Ok(guess) = text.parse::<i32>() {
            let mut lock = state.write().await;
            let chat_id = msg.chat.id.0;
            if let Some(game) = lock.by_chat.get_mut(&chat_id) {
                if game.attempts_left == 0 {
                    bot.send_message(
                        msg.chat.id,
                        "Nessun tentativo rimasto. /gioco per ricominciare.",
                    )
                    .await?;
                    return Ok(());
                }

                game.attempts_left = game.attempts_left.saturating_sub(1);

                if guess == game.target {
                    bot.send_message(msg.chat.id, "✅ Esatto! Hai indovinato! Gioco resettato.")
                        .await?;
                    // Reset partita
                    *game = GameState {
                        target: rand_in_range(1, 100),
                        attempts_left: 5,
                    };
                } else if guess < game.target {
                    bot.send_message(
                        msg.chat.id,
                        format!("Troppo basso. Tentativi rimasti: {}", game.attempts_left),
                    )
                    .await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        format!("Troppo alto. Tentativi rimasti: {}", game.attempts_left),
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}

fn rand_in_range(min: i32, max: i32) -> i32 {
    // Use the rand crate's thread-local RNG for uniform sampling
    rand::rng().random_range(min..=max)
}
