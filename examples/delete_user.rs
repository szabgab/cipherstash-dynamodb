mod common;
use crate::common::User;
use cryptonamo::encrypted_table::EncryptedTable;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    env_logger::init();

    let config = aws_config::from_env()
        .endpoint_url("http://localhost:8000")
        .load()
        .await;

    let client = aws_sdk_dynamodb::Client::new(&config);

    let table = EncryptedTable::init(client, "users").await?;

    table.delete::<User>("jane@smith.org").await?;
    table.delete::<User>("dan@coderdan.co").await?;
    table.delete::<User>("daniel@example.com").await?;

    Ok(())
}