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
    id: Thing,
}

impl UserId {
    #[tracing::instrument]
    pub async fn get_by_username(username: String) -> Option<Self> {
        let db = DB.clone();

        let mut result = db
            .query("SELECT * FROM users WHERE username = $name")
            .bind(("name", username))
            .await
            .ok()?;

        result.take(0).unwrap()
    }

    #[tracing::instrument]
    pub async fn get_by_id(id: String) -> color_eyre::Result<Option<Self>> {
        Ok(DB.select(("users", id)).await?)
    }

    #[tracing::instrument]
    pub fn id(&self) -> String {
        self.id.id.to_string()
    }

    // Get inner user

    #[tracing::instrument]
    pub async fn user(&self) -> Option<User> {
        let db = DB.clone();

        let result: Option<User> = db.select(("users", self.id().clone())).await.ok()?;

        result
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

        let existing_user = UserId::get_by_username(self.username.clone()).await;

        tracing::info!("Existing user: {:?}", existing_user);

        let user_id = match existing_user {
            Some(user_id) => {
                let user: Option<UserId> =
                    db.update(("users", user_id.id())).content(&self).await?;

                user.ok_or_eyre("User not updated")?
            }
            None => {
                let user: Option<UserId> = db
                    .create(("users", ulid::Generator::default().generate()?.to_string()))
                    .content(&self)
                    .await?;

                user.iter().next().ok_or_eyre("User not created")?.clone()
            }
        };

        Ok(user_id)
    }

    #[tracing::instrument]
    pub async fn get_by_username(username: String) -> color_eyre::Result<Option<Self>> {
        let db = DB.clone();

        let mut result = db
            .query("SELECT * FROM users WHERE username = $name")
            .bind(("name", username))
            .await?;

        Ok(result.take(0)?)
    }

    #[tracing::instrument]
    pub async fn get_by_id(id: String) -> color_eyre::Result<Option<Self>> {
        Ok(DB.select(("users", id)).await?)
    }

    #[tracing::instrument]
    pub async fn delete(&self) -> Result<()> {
        let db = DB.clone();

        if let Some(user_id) = UserId::get_by_username(self.username.clone()).await {
            let result: Option<User> = db
                .delete(("users", user_id.id()))
                .await?
                .ok_or_eyre("User not found")?;

            // this function will drop user from memory

            drop(result);
        }

        Ok(())
    }

    pub async fn id(&self) -> Option<UserId> {
        let result = UserId::get_by_username(self.username.clone()).await;
        result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageId {
    id: Thing,
}

impl MessageId {
    pub async fn new_message(user_id: String, content: String) -> color_eyre::Result<Self> {
        let db = DB.clone();
        let ulid = ulid::Generator::default().generate()?.to_string();
        let message_id: Option<Self> = db
            .create(("messages", ulid.clone()))
            .content(&Message { content })
            .await?;

        db.query(format!(
            "RELATE messages:{ulid}->sent_by->users:{user_id} SET time.sent = time::now() PARALLEL",
        ))
        .await?;

        Ok(message_id.ok_or_eyre("Message not created")?)
    }
    pub async fn get_by_id(id: String) -> color_eyre::Result<Option<Self>> {
        Ok(DB.select(("messages", id)).await?)
    }

    pub fn id(&self) -> String {
        self.id.id.to_string()
    }

    pub async fn message(&self) -> Option<Message> {
        let db = DB.clone();

        let result: Option<Message> = db.select(("messages", self.id().clone())).await.ok()?;

        result
    }

    pub async fn delete(self) -> Result<()> {
        let db = DB.clone();

        if let Some(message_id) = MessageId::get_by_id(self.id()).await? {
            let result: Option<Message> = db
                .delete(("messages", message_id.id()))
                .await?
                .ok_or_eyre("Message not found")?;

            // this function will drop message from memory

            drop(result);
        }

        Ok(())
    }

    pub async fn get_all() -> color_eyre::Result<Vec<Self>> {
        let db = DB.clone();

        let mut result = db.query("SELECT * FROM messages").await?;

        Ok(result.take(0)?)
    }

    pub async fn reply(&self, user_id: String, content: String) -> color_eyre::Result<Self> {
        let db = DB.clone();
        let message_id = Self::new_message(user_id, content).await?;

        db.query(format!(
            "RELATE messages:{reply_id}->reply_to->messages:{message_id} PARALLEL",
            reply_id = message_id.id(),
            message_id = self.id()
        ))
        .await?;

        Ok(message_id)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub content: String,
}

impl Message {
    /// Sends a message to a user
    ///
    /// Redirects to `MessageId::new_message`
    pub async fn send_message(user_id: String, content: String) -> color_eyre::Result<MessageId> {
        MessageId::new_message(user_id, content).await
    }

    pub async fn get_by_id(id: String) -> color_eyre::Result<Option<Self>> {
        Ok(DB.select(("messages", id)).await?)
    }

    pub async fn id(&self) -> Option<MessageId> {
        let result: Result<Option<MessageId>> = MessageId::get_by_id(self.content.clone()).await;
        result.ok()?
    }

    pub async fn reply(&self, user_id: String, content: String) -> color_eyre::Result<MessageId> {
        let id = self.id().await.ok_or_eyre("Message not found")?;

        id.reply(user_id, content).await
    }
}
