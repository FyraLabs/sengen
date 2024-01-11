use tracing::debug;

use crate::db::{Channel, Message, User};
mod db;
mod dbconn;

#[tokio::main]

async fn main() -> color_eyre::Result<()> {
    // tracing subscriber
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    dbconn::init_db().await?;

    tracing::debug!("Starting");

    let user = if let Some(user) = User::get_by_username("test".to_string()).await {
        user
    } else {
        User::new("test".to_string(), None).save().await?
    };

    user.save().await?;

    let channel = Channel::new("test".to_string()).create().await?;

    println!("{:?}", user);

    let msg = Message::new("Hello World!".to_string())
        .send(user.id.clone(), channel.clone())
        .await?;

    println!("{:?}", msg);

    // Deleted messages will be dropped from the database

    // msg.delete().await?;

    msg.reply(user.id(), "Hello world to you too!".to_string())
        .await?;

    let messages = channel.get_messages().await?;

    debug!("{:?}", messages);

    // channel.delete().await?;

    Ok(())
}
