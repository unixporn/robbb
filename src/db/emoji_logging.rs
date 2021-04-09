use anyhow::*;

use super::Db;

use serenity::model::id::EmojiId;

pub struct EmojiData {
    //You could store the emoji itself in here
    //    emoji: Emoji,
    reactions: u64,
    in_text: u64,
}

impl Default for EmojiData {
    fn default() -> Self {
        EmojiData {
            reactions: 0,
            in_text: 0,
        }
    }
}

impl Db {
    pub async fn increment_emoji_reaction(&self, count: u64, emoji_name: impl AsRef<str>, emoji: &EmojiId) -> Result<EmojiData> {
        let mut data = self.get_emoji_usage(emoji).await?;
        let mut conn = self.pool.acquire().await?;
        let emoji_str = emoji_name.as_ref();
        data.reactions += count;
        let num = data.reactions as i64;
        let id = emoji.0 as i64;
        sqlx::query!("insert into emojis (emoji_id, emoji_name, reaction_usage) values (?1, ?2, ?3) on conflict(emoji_id) do update set reaction_usage=?3", id, emoji_str,num).execute(&mut conn).await?;
        Ok(data)
    }

    pub async fn increment_emoji_text(&self, count: u64, emoji_name : impl AsRef<str> , emoji: &EmojiId) -> Result<EmojiData> {
        let mut data = self.get_emoji_usage(emoji).await?;
        let mut conn = self.pool.acquire().await?;
        let emoji_str = emoji_name.as_ref();
        data.in_text += count;
        let num = data.in_text as i64;
        let id = emoji.0 as i64;
        sqlx::query!("insert into emojis (emoji_id, emoji_name, in_text_usage) values (?1, ?2, ?3) on conflict(emoji_id) do update set in_text_usage=?3",id, emoji_str,num).execute(&mut conn).await?;
        Ok(data)
    }

    pub async fn get_emoji_usage(&self, emoji: &EmojiId) -> Result<EmojiData> {
        let mut conn = self.pool.acquire().await?;
        let emoji_id = emoji.0 as i64;
        let value = sqlx::query!("select * from emojis where emoji_id=?", emoji_id)
            .fetch_optional(&mut conn)
            .await?;
        match value {
            Some(x) => {
                return Ok(EmojiData {
                    //                   emoji: emoji.clone(),
                    in_text: x.in_text_usage as u64,
                    reactions: x.reaction_usage as u64,
                });
            }
            None => return Ok(EmojiData::default()),
        }
    }
}
