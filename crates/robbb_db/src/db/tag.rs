use chrono::{DateTime, Utc};
use eyre::Result;
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct Tag {
    pub name: String,
    pub moderator: UserId,
    pub content: String,
    pub official: bool,
    pub create_date: Option<DateTime<Utc>>,
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn set_tag(
        &self,
        moderator: UserId,
        name: String,
        content: String,
        official: bool,
        create_date: Option<DateTime<Utc>>,
    ) -> Result<Tag> {
        let moderator_id: i64 = moderator.into();
        sqlx::query!(
            "insert into tag (name, moderator, content, official, create_date) values (?, ?, ?, ?, ?)
                on conflict(name) do update set moderator=?, content=?, official=?, create_date=?",
            name,
            moderator_id,
            content,
            official,
            create_date,
            moderator_id,
            content,
            official,
            create_date,
        )
            .execute(&self.pool)
            .await?;

        // Insert into the cache if there are already things in the cache.
        // If there aren't yet, then we don't care, as the cache will be filled with all values when it's read for the first time.
        if let Some(tag_names) = self.tag_name_cache.write().await.as_mut() {
            tag_names.insert(name.clone());
        }

        Ok(Tag { name, moderator, content, official, create_date })
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_tag(&self, name: &str) -> Result<Option<Tag>> {
        Ok(sqlx::query!(
            r#"select name as "name!", moderator, content, official, create_date from tag where name=? COLLATE NOCASE"#,
            name
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|x| {
            let create_date = x
                .create_date
                .map(|date| chrono::DateTime::from_naive_utc_and_offset(date, chrono::Utc));
            Tag {
            name: x.name,
            moderator: UserId::new(x.moderator as u64),
            content: x.content,
            official: x.official,
            create_date,
        }}))
    }

    #[tracing::instrument(skip_all)]
    pub async fn delete_tag(&self, name: String) -> Result<()> {
        sqlx::query!(r#"delete from tag where name=? COLLATE NOCASE"#, name)
            .execute(&self.pool)
            .await?;

        if let Some(tag_names) = self.tag_name_cache.write().await.as_mut() {
            tag_names.remove(&name);
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn list_tags(&self) -> Result<Vec<String>> {
        let tag_name_cache = self.tag_name_cache.read().await;
        if let Some(tag_names) = tag_name_cache.as_ref() {
            Ok(tag_names.clone().into_iter().collect())
        } else {
            std::mem::drop(tag_name_cache);
            let tag_names = sqlx::query_scalar(r#"select name as "name!" from tag"#)
                .fetch_all(&self.pool)
                .await?;

            let mut tag_name_cache = self.tag_name_cache.write().await;
            let _ = tag_name_cache.insert(tag_names.clone().into_iter().collect());

            Ok(tag_names)
        }
    }
}
