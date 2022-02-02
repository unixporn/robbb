use anyhow::Result;
use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct Kick {
    pub id: i64,
    pub moderator: UserId,
    pub user: UserId,
    pub reason: String,
    pub create_date: DateTime<Utc>,
    pub context: Option<String>,
}

impl Db {
    pub async fn add_kick(
        &self,
        moderator: UserId,
        user: UserId,
        reason: String,
        create_date: DateTime<Utc>,
        context: Option<String>,
    ) -> Result<Kick> {
        let mut conn = self.pool.acquire().await?;

        let id = {
            let moderator = moderator.0 as i64;
            let user = user.0 as i64;
            sqlx::query!(
                "insert into kick (moderator, usr, reason, create_date, context) values(?, ?, ?, ?, ?)",
                moderator,
                user,
                reason,
                create_date,
                context,
            )
            .execute(&mut conn)
            .await?
            .last_insert_rowid()
        };

        Ok(Kick {
            id,
            moderator,
            user,
            reason,
            create_date,
            context,
        })
    }

    pub async fn get_kicks(&self, user: UserId) -> Result<Vec<Kick>> {
        let mut conn = self.pool.acquire().await?;
        let id = user.0 as i64;
        Ok(sqlx::query!(
            r#"select id, moderator, usr, reason as "reason!", create_date as "create_date!", context from kick where usr=?"#,
            id
        )
        .fetch_all(&mut conn)
        .await?
        .into_iter()
        .map(|x| Kick {
            id: x.id,
            moderator: UserId(x.moderator as u64),
            user: UserId(x.usr as u64),
            reason: x.reason,
            create_date: chrono::DateTime::from_utc(x.create_date, Utc),
            context: x.context,
        }).collect())
    }
}
