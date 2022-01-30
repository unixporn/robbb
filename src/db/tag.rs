use anyhow::Result;
use chrono::{DateTime, Utc};
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
    pub async fn set_tag(
        &self,
        moderator: UserId,
        name: String,
        content: String,
        official: bool,
        create_date: Option<DateTime<Utc>>,
    ) -> Result<Tag> {
        let mut conn = self.pool.acquire().await?;

        let moderator_id = moderator.0 as i64;
        sqlx::query!(
            "insert into tag (name, moderator, content, official, create_date) values (?, ?, ?, ?, ?) on conflict(name) do update set moderator=?, content=?, official=?, create_date=?",
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
            .execute(&mut conn)
            .await?;

        Ok(Tag {
            name,
            moderator,
            content,
            official,
            create_date,
        })
    }

    pub async fn get_tag(&self, name: &str) -> Result<Option<Tag>> {
        let mut conn = self.pool.acquire().await?;
        Ok(sqlx::query!(
            r#"select name as "name!", moderator, content, official, create_date from tag where name=? COLLATE NOCASE"#,
            name
        )
        .fetch_optional(&mut conn)
        .await?
        .map(|x| {
            let create_date = x
                .create_date
                .map(|date| chrono::DateTime::from_utc(date, chrono::Utc));
            Tag {
            name: x.name,
            moderator: UserId(x.moderator as u64),
            content: x.content,
            official: x.official,
            create_date,
        }}))
    }

    pub async fn delete_tag(&self, name: String) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!(r#"delete from tag where name=? COLLATE NOCASE"#, name)
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn list_tags(&self) -> Result<Vec<String>> {
        let mut conn = self.pool.acquire().await?;
        Ok(sqlx::query_scalar(r#"select name as "name!" from tag"#)
            .fetch_all(&mut conn)
            .await?)
    }
}
