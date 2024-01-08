use surrealdb::{engine::remote::ws::Ws, opt::auth::Root};

mod db;
mod dbconn;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let surrealdb = surrealdb::Surreal::new::<Ws>(&database_url).await?;

    surrealdb
        .signin(Root {
            username: "root",
            password: "root",
        })
        .await?;

    Ok(())
}
