use crate::db::User;
mod db;
mod dbconn;

#[tokio::main]

async fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();
    dbconn::init_db().await?;

    let mut user = User::new("test".to_string(), None);

    let user = user.save().await?;

    println!("{:?}", user);

    Ok(())
}
