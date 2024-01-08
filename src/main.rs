mod db;
mod dbconn;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();
    let surrealdb = &dbconn::get_conn().await?.surrealdb;


    let health = surrealdb.health();

    println!("{:?}", health.await?);

    Ok(())
}
