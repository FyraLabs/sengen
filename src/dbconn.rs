use once_cell::sync::Lazy;
use surrealdb::engine::remote::ws::{Client, Ws, Wss};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

pub struct EnvConfig {
    pub database_url: String,
    pub username: String,
    pub password: String,
    pub database: String,
    pub namespace: String,
}

impl EnvConfig {
    pub fn infer_env() -> color_eyre::Result<Self> {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let username = std::env::var("SURREAL_USER").expect("SURREAL_USER must be set");
        let password = std::env::var("SURREAL_PASS").expect("SURREAL_PASS must be set");
        let database = std::env::var("SURREAL_DATABASE").expect("SURREAL_DATABASE must be set");
        let namespace = std::env::var("SURREAL_NAMESPACE").expect("SURREAL_NAMESPACE must be set");

        Ok(Self {
            database_url,
            username,
            password,
            database,
            namespace,
        })
    }
}

pub static DB: Lazy<Surreal<Client>> = Lazy::new(Surreal::init);

pub async fn init_db() -> color_eyre::Result<()> {
    let config = EnvConfig::infer_env()?;

    DB.connect::<Ws>(config.database_url).await?;
    DB.signin(Root {
        username: &config.username,
        password: &config.password,
    })
    .await?;

    DB.use_ns(&config.namespace)
        .use_db(&config.database)
        .await?;

    Ok(())
}
