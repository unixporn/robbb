use anyhow::*;

use super::Db;

use serenity::model::{id::EmojiId, misc::EmojiIdentifier};

pub struct EmojiData {
    emoji: EmojiIdentifier,
    reactions: u64,
    in_text: u64,
}

impl EmojiData {
    pub fn new(emoji_id: EmojiIdentifier) -> EmojiData {
        EmojiData {
            emoji: emoji_id,
            reactions: 0,
            in_text: 0,
        }
    }
}

impl Db {
    pub async fn increment_emoji_reaction(
        &self,
        count: u64,
        emoji_name: impl AsRef<str>,
        emoji: &EmojiId,
    ) -> Result<EmojiData> {
        let mut conn = self.pool.acquire().await?;
        let emoji_str = emoji_name.as_ref();
        let id = emoji.0 as i64;
        let count = count as i64;
        sqlx::query!("insert into emojis (emoji_id, emoji_name, reaction_usage) values (?1, ?2, ?3) on conflict(emoji_id) do update set reaction_usage=reaction_usage+?3",
            id, emoji_str, count)
            .execute(&mut conn)
            .await?;
        Ok(self.get_emoji_usage(emoji).await?)
    }

    pub async fn increment_emoji_text(
        &self,
        count: u64,
        emoji_name: impl AsRef<str>,
        emoji: &EmojiId,
    ) -> Result<EmojiData> {
        let mut conn = self.pool.acquire().await?;
        let id = emoji.0 as i64;
        let emoji_str = emoji_name.as_ref();
        let count = count as i64;
        sqlx::query!("insert into emojis (emoji_id, emoji_name, in_text_usage) values (?1, ?2, ?3) on conflict(emoji_id) do update set in_text_usage=in_text_usage+?3",
            id, emoji_str, count)
            .execute(&mut conn)
            .await?;
        Ok(self.get_emoji_usage(emoji).await?)
    }

    pub async fn get_emoji_usage(&self, emoji: &EmojiId) -> Result<EmojiData> {
        let mut conn = self.pool.acquire().await?;
        let emoji_id = emoji.0 as i64;
        let value = sqlx::query!("select * from emojis where emoji_id=?", emoji_id)
            .fetch_optional(&mut conn)
            .await?;
        Ok(value
            .map(|x| EmojiData {
                in_text: x.in_text_usage as u64,
                reactions: x.reaction_usage as u64,
            })
            .unwrap_or_default())
    }

    pub async fn get_all_emojis(&self) -> Result<Vec<EmojiData>> {
        let mut conn = self.pool.acquire().await?;
        let records = sqlx::query!("select * from emojis")
            .fetch_all(&mut conn)
            .await?;
        Ok(records.into_iter().map(|))
    }
}
