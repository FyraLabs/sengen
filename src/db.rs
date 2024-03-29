use crate::dbconn::DB;
use color_eyre::eyre::{anyhow, OptionExt};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surreal_id::NewId;
use surrealdb::opt::RecordId;
use surrealdb::sql::Thing;
use tracing::{debug, info};
use ulid::Ulid;

/// Simply generates a new ULID
/// Will be a function here even though it's simply a wrapper because
/// some day we might want to change the implementation
pub fn ulid() -> Ulid {
    Ulid::new()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserId(RecordId);

impl NewId for UserId {
    const TABLE: &'static str = "users";

    fn from_inner_id<T: Into<surrealdb::sql::Id>>(inner_id: T) -> Self {
        Self(RecordId {
            tb: Self::TABLE.to_string(),
            id: inner_id.into(),
        })
    }

    fn get_inner_string(&self) -> String {
        self.0.id.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub nickname: Option<String>,
}

impl User {
    /// Creates a new user
    #[tracing::instrument]
    pub fn new(username: String, nickname: Option<String>) -> Self {
        let ulid = ulid();

        Self {
            username,
            nickname,
            id: UserId::new(ulid.to_string()).unwrap(),
        }
    }

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

    pub fn get_display_name(&self) -> String {
        match &self.nickname {
            Some(nickname) => nickname.clone(),
            None => self.username.clone(),
        }
    }

    pub fn get_username(&self) -> String {
        self.username.clone()
    }

    #[tracing::instrument]
    pub async fn save(&self) -> color_eyre::Result<Self> {
        let db = DB.clone();

        // get user by username if exists, then update

        let existing_user = User::get_by_username(self.username.clone()).await;

        tracing::info!("Existing user: {:?}", existing_user);

        let user = match existing_user {
            Some(user) => {
                let user: Option<Self> = db
                    .update(("users", user.id.0.clone()))
                    .content(&self)
                    .await?;

                user.ok_or_eyre("User not updated")?
            }
            None => {
                let user_id: Option<Self> = db
                    .create(("users", self.id.0.clone()))
                    .content(&self)
                    .await?;

                user_id.ok_or_eyre("User not created")?
            }
        };

        Ok(user)
    }

    #[tracing::instrument]
    pub async fn get_by_id(id: String) -> color_eyre::Result<Option<Self>> {
        Ok(DB.select(("users", id)).await?)
    }

    #[tracing::instrument]
    pub async fn delete(&self) -> Result<()> {
        let db = DB.clone();

        if let Some(user) = User::get_by_username(self.username.clone()).await {
            let result: Option<User> = db
                .delete(("users", user.id.0.id))
                .await?
                .ok_or_eyre("User not found")?;

            // this function will drop user from memory

            drop(result);
        }

        Ok(())
    }

    pub fn id(&self) -> String {
        self.id.0.id.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageId(RecordId);

impl NewId for MessageId {
    const TABLE: &'static str = "messages";

    fn from_inner_id<T: Into<surrealdb::sql::Id>>(inner_id: T) -> Self {
        Self(RecordId {
            tb: Self::TABLE.to_string(),
            id: inner_id.into(),
        })
    }

    fn get_inner_string(&self) -> String {
        self.0.id.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: MessageId,
    pub content: String,
}

impl Message {
    #[tracing::instrument]
    pub fn new(content: String) -> Self {
        let ulid = ulid();

        Self {
            content,
            id: MessageId::new(ulid.to_string()).unwrap(),
        }
    }

    pub fn get_content(&self) -> String {
        self.content.clone()
    }

    #[tracing::instrument]
    pub async fn send(&self, user_id: UserId, channel: Channel) -> Result<Self> {
        let db = DB.clone();

        let msgid = self.id.0.clone();

        let message_id: Option<Self> = db
            .create(("messages", msgid.clone()))
            .content(&self)
            .await?;

        db.query(format!(
            "RELATE ONLY messages:{ulid}->sent_by->users:{user_id} SET time.sent = time::now() PARALLEL",
            ulid = msgid.id,
            user_id = user_id.get_inner_string()
        ))
        .await?;

        db.query(format!(
            "RELATE ONLY messages:{ulid}->sent_in_channel->channels:{channel_id} PARALLEL",
            ulid = msgid.id,
            channel_id = channel.id()
        ))
        .await?;

        Ok(message_id.ok_or_eyre("Message not created")?)
    }

    #[tracing::instrument]
    pub async fn get_by_id(id: String) -> color_eyre::Result<Option<Self>> {
        Ok(DB.select(("messages", id)).await?)
    }

    #[tracing::instrument]
    pub fn id(&self) -> String {
        self.id.0.id.to_string()
    }

    #[tracing::instrument]
    pub async fn reply(&self, user_id: String, content: String) -> color_eyre::Result<Self> {
        let db = DB.clone();

        // get channel from message

        let channel = db
            .query(format!(
                "RETURN (SELECT * FROM sent_in_channel WHERE in = messages:{id}).out FETCH out",
                id = self.id()
            ))
            .await?
            .take(0);

        let channel: Option<Channel> = channel?;

        let new_message = Message::new(content)
            .send(
                UserId::from_inner_id(user_id),
                channel.ok_or_eyre("Channel not found")?,
            )
            .await?;

        db.query(format!(
            "RELATE messages:{reply_id}->reply_to->messages:{message_id} PARALLEL",
            reply_id = self.id(),
            message_id = new_message.id()
        ))
        .await?;

        Ok(new_message)
    }

    /// Get the message this message was replied from
    #[tracing::instrument]
    pub async fn replied_from(&self) -> color_eyre::Result<Option<Self>> {
        let db = DB.clone();

        let result: Option<Message> = db
            .query(format!(
                "RETURN (SELECT * FROM reply_to WHERE out = messages:{id}).in FETCH in",
                id = self.id()
            ))
            .await?
            .take(0)?;

        Ok(result)
    }

    #[tracing::instrument]
    pub async fn get_replies(&self) -> color_eyre::Result<Vec<Self>> {
        let db = DB.clone();

        let result: Vec<Message> = db
            .query(format!(
                "RETURN (SELECT * FROM reply_to WHERE in = messages:{id}).out FETCH out",
                id = self.id()
            ))
            .await?
            .take(0)?;

        Ok(result)
    }

    #[tracing::instrument]
    pub async fn delete(self) -> Result<()> {
        let db = DB.clone();

        let _result: Self = db
            .delete(("messages", self.clone().id.0.id))
            .await?
            .ok_or_eyre("Unable to delete message")?;

        Ok(())
    }

    #[tracing::instrument]
    pub async fn get_channel(&self) -> color_eyre::Result<Channel> {
        let db = DB.clone();

        let result: Option<Channel> = db
            .query(format!(
                "RETURN (SELECT * FROM sent_in_channel WHERE in = messages:{id}).out FETCH out",
                id = self.id()
            ))
            .await?
            .take(0)?;

        Ok(result.ok_or_eyre("Channel not found!?")?)
    }

    #[tracing::instrument]
    pub async fn get_sender(&self) -> color_eyre::Result<User> {
        let db = DB.clone();

        let result: Option<User> = db
            .query(format!(
                "RETURN (SELECT * FROM sent_by WHERE in = messages:{id}).out FETCH out",
                id = self.id()
            ))
            .await?
            .take(0)?;

        Ok(result.ok_or_eyre("User not found!?")?)
    }

    /// Get the time this message was sent
    #[tracing::instrument]
    pub async fn get_time_sent(&self) -> color_eyre::Result<chrono::DateTime<chrono::Utc>> {
        let db = DB.clone();

        let result: Option<chrono::DateTime<chrono::Utc>> = db
            .query(format!(
                "RETURN (SELECT * FROM sent_by WHERE in = messages:{id}).time.sent FETCH time.sent",
                id = self.id()
            ))
            .await?
            .take(0)?;

        Ok(result.ok_or_eyre("Time not found!?")?)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChannelId(RecordId);

impl NewId for ChannelId {
    const TABLE: &'static str = "channels";

    fn from_inner_id<T: Into<surrealdb::sql::Id>>(inner_id: T) -> Self {
        Self(RecordId {
            tb: Self::TABLE.to_string(),
            id: inner_id.into(),
        })
    }

    fn get_inner_string(&self) -> String {
        self.0.id.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
}

impl Channel {
    /// Creates a new channel instance
    #[tracing::instrument]
    pub fn new(name: String) -> Self {
        let ulid = ulid();

        Self {
            name,
            id: ChannelId::new(ulid.to_string()).unwrap(),
        }
    }

    #[tracing::instrument]
    pub async fn create(&self) -> Result<Self> {
        let db = DB.clone();

        let channel_id: Option<Self> = db
            .create(("channels", self.id.0.clone()))
            .content(&self)
            .await?;

        Ok(channel_id.ok_or_eyre("Channel not created")?)
    }

    #[tracing::instrument]
    pub async fn get_by_id(id: Ulid) -> color_eyre::Result<Option<Self>> {
        // Convert Ulid to surrealdb::sql::Id
        Ok(DB.select(("channels", id.to_string())).await?)
    }

    #[tracing::instrument]
    pub async fn get_by_name(name: String) -> color_eyre::Result<Option<Self>> {
        let db = DB.clone();

        let result: Option<Channel> = db
            .query("SELECT * FROM channels WHERE name = $name")
            .bind(("name", name))
            .await?
            .take(0)?;

        Ok(result)
    }

    pub fn id(&self) -> String {
        self.id.0.id.to_string()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[tracing::instrument]
    pub async fn delete(self) -> Result<()> {
        let db = DB.clone();
        info!("Deleting messages in channel");
        let channel_id = self.id.0.id.to_string();

        info!("Deleting channel");

        let _result: Self = db
            .delete(("channels", self.clone().id.0.id))
            .await?
            .ok_or_eyre("Unable to delete channel")?;

        info!("Channel deleted");

        info!("Cleaning up messages in channel");

        tracing::span!(tracing::Level::INFO, "delete_messages", channel = %self.id())
            .in_scope(|| async move {
                let _ = &DB
                    .clone()
                    .query(format!(
                        "DELETE messages WHERE channel.id = channels:{id} PARALLEL",
                        id = channel_id
                    ))
                    .await
                    .unwrap();
            })
            .await;

        Ok(())
    }

    #[tracing::instrument]
    pub async fn get_messages(&self) -> color_eyre::Result<Vec<Message>> {
        let db = DB.clone();

        let mut result = db
            .query(format!(
                r#"
                RETURN (SELECT * FROM sent_in_channel WHERE out = channels:{id}).in FETCH in
                "#,
                id = self.id()
            ))
            .bind(("id", self.id()))
            .await?;

        debug!("Messages: {:?}", result);

        // let data = result.take(0)?;

        Ok(result.take(0)?)
    }
}
