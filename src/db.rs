use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct User {
    pub id: Uuid,
    pub username: String,
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

impl User {
    pub fn new(username: String, nickname: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            nickname,
        }
    }


    pub fn get_display_name(&self) -> String {
        match &self.nickname {
            Some(nickname) => nickname.clone(),
            None => self.username.clone(),
        }
    }
}

