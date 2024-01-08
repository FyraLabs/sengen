use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use tokio::sync::OnceCell;

pub struct EnvConfig {
    pub database_url: String,
    pub username: String,
    pub password: String,
    pub database: String,
    pub namespace: String,
}

pub struct DbConn {
    pub surrealdb: Surreal<Client>,
    pub id: usize,
}

pub struct DbConnPool {
    pub conns: Vec<DbConn>,
}

impl DbConnPool {
    pub fn get_conn(&self) -> Option<&DbConn> {
        // get random conn

        // todo: probably do round robin instead of random

        let mut rng = rand::thread_rng();

        let index = rand::Rng::gen_range(&mut rng, 0..self.conns.len());

        tracing::trace!(conn_id = ?index, "Got connection from pool");
        self.conns.get(index)
    }
}

impl EnvConfig {
    pub fn infer_env() -> color_eyre::Result<Self> {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let username = std::env::var("SURREAL_USERNAME").expect("SURREAL_USERNAME must be set");
        let password = std::env::var("SURREAL_PASSWORD").expect("SURREAL_PASSWORD must be set");
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
/// Connect to the database using the environment variables.
///
/// It's recommended to not use this function directly, but instead use `DbConnPool::get_conn()` or `get_pool()` to get a connection from the pool.
///
/// A valid case to use this function directly is if you want to actually start off a new connection thread for a heavy task(?)
pub async fn connect_env() -> color_eyre::Result<Surreal<Client>> {
    let config = EnvConfig::infer_env()?;
    let surrealdb = Surreal::new::<Ws>(&config.database_url).await?;

    surrealdb
        .signin(Root {
            username: &config.username,
            password: &config.password,
        })
        .await?;

    surrealdb
        .use_ns(&config.namespace)
        .use_db(&config.database)
        .await?;

    tracing::debug!("Connected to database successfully");

    Ok(surrealdb)
}

const DEFAULT_POOL_SIZE: usize = 10;

async fn create_pool() -> color_eyre::Result<DbConnPool> {
    let mut conns = Vec::new();

    for _ in 0..DEFAULT_POOL_SIZE {
        let surrealdb = connect_env().await?;
        let id = rand::random::<usize>();

        conns.push(DbConn { surrealdb, id });
    }

    Ok(DbConnPool { conns })
}

static POOL: OnceCell<DbConnPool> = OnceCell::const_new();

async fn get_pool() -> color_eyre::Result<&'static DbConnPool> {
    POOL.get_or_try_init(create_pool).await
}

pub async fn get_conn() -> color_eyre::Result<&'static DbConn> {
    let pool = get_pool().await?;
    pool.get_conn()
        .ok_or_else(|| color_eyre::eyre::anyhow!("No connections in pool"))
}
