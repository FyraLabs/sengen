use color_eyre::eyre::{anyhow, OptionExt};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use uuid::Uuid;

use crate::dbconn::DB;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserId {
    pub id: Thing,
}

impl UserId {
    pub async fn get_by_username(username: String) -> color_eyre::Result<Option<Self>> {
        let db = DB.clone();

        let mut result = db
            .query("SELECT * FROM users WHERE username = $name")
            .bind(("name", username))
            .await?;

        Ok(result.take(0)?)
    }

    pub async fn get_by_id(id: String) -> color_eyre::Result<Option<Self>> {
        Ok(DB.select(("users", id)).await?)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub username: String,
    pub nickname: Option<String>,
}

impl User {
    pub fn new(username: String, nickname: Option<String>) -> Self {
        Self { username, nickname }
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

    pub async fn save(&self) -> color_eyre::Result<UserId> {
        let db = DB.clone();

        // get user by username if exists, then update

        let existing_user = UserId::get_by_username(self.username.clone()).await?;

        tracing::info!("Existing user: {:?}", existing_user);

        let user_id = match existing_user {
            Some(user_id) => {
                let user: Option<UserId> = db
                    .update(("users", user_id.id.id))
                    .content(&self)
                    .await?;

                user.ok_or_eyre("User not updated")?
            }
            None => {
                let user: Vec<UserId> = db.create("users").content(&self).await?;

                user.iter().next().ok_or_eyre("User not created")?.clone()
            }
        };

        Ok(user_id)
    }
}
