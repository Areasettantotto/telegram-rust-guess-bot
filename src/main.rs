use telegram_bot_rust::run_bot;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_bot().await
}
