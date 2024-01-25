use anyhow::{Context, Result};

use super::Db;

use serenity::model::id::EmojiId;

pub struct EmojiStats {
    pub emoji: EmojiIdentifier,
    pub reactions: u64,
    pub in_text: u64,
}

impl EmojiStats {
    pub fn new(emoji_id: EmojiIdentifier) -> EmojiStats {
        EmojiStats { emoji: emoji_id, reactions: 0, in_text: 0 }
    }
}

#[derive(Clone)]
pub struct EmojiIdentifier {
    pub id: EmojiId,
    pub animated: bool,
    pub name: String,
}

pub enum Ordering {
    Ascending,
    Descending,
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn alter_emoji_reaction_count(
        &self,
        amount: i64,
        emoji: &EmojiIdentifier,
    ) -> Result<EmojiStats> {
        let emoji_str = &emoji.name;
        let id: i64 = emoji.id.into();
        sqlx::query!("insert into emoji_stats (emoji_id, emoji_name, reaction_usage, animated) values (?1, ?2, max(0, ?3), ?4) on conflict(emoji_id) do update set reaction_usage=max(0, reaction_usage + ?3)",
            id, emoji_str, amount, emoji.animated)
            .execute(&self.pool)
            .await?;
        self.get_emoji_usage_by_id(emoji).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn alter_emoji_text_count(
        &self,
        amount: i64,
        emoji: &EmojiIdentifier,
    ) -> Result<EmojiStats> {
        let id: i64 = emoji.id.into();
        let emoji_str = &emoji.name;
        sqlx::query!("insert into emoji_stats (emoji_id, emoji_name, in_text_usage, animated) values (?1, ?2, max(0, ?3), ?4) on conflict(emoji_id) do update set in_text_usage=max(0, in_text_usage + ?3)",
            id, emoji_str, amount, emoji.animated)
            .execute(&self.pool)
            .await?;
        self.get_emoji_usage_by_id(emoji).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_emoji_usage_by_id(&self, emoji: &EmojiIdentifier) -> Result<EmojiStats> {
        let emoji_id: i64 = emoji.id.into();
        let value = sqlx::query!("select * from emoji_stats where emoji_id=?", emoji_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(value
            .map(|x| EmojiStats {
                emoji: EmojiIdentifier {
                    id: EmojiId::new(x.emoji_id as u64),
                    animated: x.animated != 0,
                    name: x.emoji_name.unwrap(),
                },
                in_text: x.in_text_usage as u64,
                reactions: x.reaction_usage as u64,
            })
            .unwrap_or_else(|| EmojiStats::new(emoji.clone())))
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_emoji_usage_by_name(&self, emoji: &str) -> Result<EmojiStats> {
        let value = sqlx::query!("select * from emoji_stats where emoji_name=?", emoji)
            .fetch_optional(&self.pool)
            .await?;
        value
            .map(|x| EmojiStats {
                emoji: EmojiIdentifier {
                    id: EmojiId::new(x.emoji_id as u64),
                    animated: x.animated != 0,
                    name: x.emoji_name.unwrap(),
                },
                in_text: x.in_text_usage as u64,
                reactions: x.reaction_usage as u64,
            })
            .context("Could not find emoji by that name")
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_top_emoji_stats(
        &self,
        count: u16,
        ordering: Ordering,
    ) -> Result<Vec<EmojiStats>> {
        // This exists to allow generic creation of queries, as the queries are two distinct types
        // and cannot be used in a match without also constructing the struct
        macro_rules! process_emoji_stats_query {
            ($query:expr,$limit:tt) => {{
                let records = sqlx::query!($query, $limit).fetch_all(&self.pool).await?;

                Ok(records
                    .into_iter()
                    .map(|x| EmojiStats {
                        emoji: EmojiIdentifier {
                            id: EmojiId::new(x.emoji_id as u64),
                            animated: x.animated != 0,
                            name: x.emoji_name.unwrap(),
                        },
                        in_text: x.in_text_usage as u64,
                        reactions: x.reaction_usage as u64,
                    })
                    .collect())
            }};
        }
        match ordering {
            Ordering::Ascending => process_emoji_stats_query!(
                r#"select *, in_text_usage + reaction_usage as "usage!: i32" FROM emoji_stats order by "usage!: i32" ASC limit ?"#,
                count
            ),
            Ordering::Descending => process_emoji_stats_query!(
                r#"select *, in_text_usage + reaction_usage as "usage!: i32" FROM emoji_stats order by "usage!: i32" DESC limit ?"#,
                count
            ),
        }
    }
}
