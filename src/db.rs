
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use uuid::Uuid;

use crate::dbconn::DB;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub username: String,
    pub nickname: Option<String>,
}

impl User {
    pub fn new(username: String, nickname: Option<String>) -> Self {
        Self { username, nickname }
    }

    pub async fn create_user(username: String) -> Self {
        let user = Self {
            username,
            nickname: None,
        };

        let db = DB.clone();

        let user: Vec<Self> db.create(("users", uuid::Uuid::new_v4().to_string())).content(&user).await?;

        user
    }

    pub fn get_display_name(&self) -> String {
        match &self.nickname {
            Some(nickname) => nickname.clone(),
            None => self.username.clone(),
        }
    }

    pub fn get_username(&self) -> String {
        self.username.clone()
    }

    pub async fn save(&self) -> color_eyre::Result<()> {
        let db = DB.clone();

        // get user by username if exists
        // wtf how does surreal select by field
        let record: Option<User> = db.select(("users", "username")).await?;

        let user: Vec<Self> = db.create("users").content(self).await?;

        tracing::info!("{:?}", user);

        Ok(())
    }

    pub async fn get_by_username(username: String) -> color_eyre::Result<Option<Self>> {
        let db = DB.clone();

        let record: Option<Self> = db.select(("users", username)).await?;

        Ok(record)
    }
}
