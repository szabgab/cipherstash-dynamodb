mod common;
use crate::common::User;
use cryptonamo::EncryptedTable;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unimplemented!();

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
    //let user: Option<User> = table.get("dan@coderdan.co").await?;

    //dbg!(user);

    Ok(())
}
