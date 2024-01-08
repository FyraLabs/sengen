use crate::db::User;
mod db;
mod dbconn;

#[tokio::main]

async fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();
    dbconn::init_db().await?;
    let db = dbconn::DB.clone();

    let user = User::get_by_username("test".to_string()).await?;

    println!("{:?}", user);

    Ok(())
}
