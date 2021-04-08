use anyhow::*;

use super::Db;

use serenity::model::guild::Emoji;

pub struct EmojiData {
    emoji: Emoji,
    reactions: u64,
    in_text: u64,
}

impl EmojiData {

    pub fn new(emoji : Emoji) -> EmojiData{
        EmojiData { emoji, reactions : 0, in_text : 0}
    }
}


impl Db {
    pub async fn increment_emoji_reaction(&self, emoji: &Emoji) -> Result<EmojiData> {
        let mut data = self.get_emoji_usage(emoji).await?;
        let mut conn = self.pool.acquire().await?;
        let emoji_str = emoji.name.clone();
        data.reactions = data.reactions+1;
        let num = data.reactions as i64;
        sqlx::query!("insert into emojis (emoji,  reaction_usage) values (?1, ?2) on conflict(emoji) do update set reaction_usage=?2", emoji_str,num).execute(&mut conn).await?;
        Ok(data)

    }

    pub async fn increment_emoji_text(&self, emoji: &Emoji) -> Result<EmojiData> {
        let mut data = self.get_emoji_usage(emoji).await?;
        let mut conn = self.pool.acquire().await?;
        let emoji_str = emoji.name.clone();
        data.in_text= data.in_text+1;
        let num = data.in_text as i64;
        sqlx::query!("insert into emojis (emoji,  in_text_usage) values (?1, ?2) on conflict(emoji) do update set in_text_usage=?2", emoji_str,num).execute(&mut conn).await?;
        Ok(data)
    }

    pub async fn get_emoji_usage(&self, emoji: &Emoji) -> Result<EmojiData> {
        let mut conn = self.pool.acquire().await?;
        let emoji_text = emoji.name.clone();
        let value = sqlx::query!("select * from emojis where emoji=?", emoji_text)
            .fetch_optional(&mut conn)
            .await?;
        match value {
            Some(x) => {
                return Ok(EmojiData{
                    emoji: emoji.clone(),
                    in_text : x.in_text_usage as u64,
                    reactions : x.reaction_usage as u64
                })
            },
            None => return Ok(EmojiData::new(emoji.clone()))
        }
    }
}
