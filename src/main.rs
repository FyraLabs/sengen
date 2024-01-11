use crate::db::{Message, User};
mod db;
mod dbconn;

#[tokio::main]

async fn main() -> color_eyre::Result<()> {

    // tracing subscriber
    tracing_subscriber::fmt::fmt().init();
    dotenvy::dotenv().ok();
    dbconn::init_db().await?;

    let user = User::new("test".to_string(), None).save().await?;

    println!("{:?}", user);

    let msg = Message::send_message(user.id(), "Hello, World!".to_string()).await?;

    println!("{:?}", msg.message().await);

    // Deleted messages will be dropped from the database

    msg.delete().await?;

    Ok(())
}
