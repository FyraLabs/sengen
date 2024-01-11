use crate::db::{ChannelId, Message, User, MessageId};
mod db;
mod dbconn;

#[tokio::main]

async fn main() -> color_eyre::Result<()> {
    // tracing subscriber
    tracing_subscriber::fmt::fmt().init();
    dotenvy::dotenv().ok();
    dbconn::init_db().await?;

    let user = User::new("test".to_string(), None).save().await?;

    let channel = ChannelId::new_channel("test".to_string()).await?;

    println!("{:?}", user);

    let msg = MessageId::new_message(user.id(), "Hello, World!".to_string(), channel.id()).await?;

    println!("{:?}", msg.message().await);

    // Deleted messages will be dropped from the database

    // msg.delete().await?;

    msg.reply(user.id(), "Hello world to you too!".to_string())
        .await?;

    
    let messages = channel.get_messages_id().await?;
    println!("{:?}", messages);

    let messages = channel.get_messages().await?;

    println!("{:?}", messages);

    Ok(())
}
