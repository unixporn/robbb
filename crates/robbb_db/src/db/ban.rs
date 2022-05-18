use anyhow::Result;
use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct Ban {
    pub id: i64,
    pub moderator: UserId,
    pub user: UserId,
    pub reason: String,
    pub create_date: DateTime<Utc>,
    pub context: Option<String>,
}

impl Db {
    pub async fn add_ban(
        &self,
        moderator: UserId,
        user: UserId,
        reason: String,
        create_date: DateTime<Utc>,
        context: Option<String>,
    ) -> Result<Ban> {
        let mut conn = self.pool.acquire().await?;

        let id = {
            let moderator = moderator.0 as i64;
            let user = user.0 as i64;
            sqlx::query!(
                "insert into ban (moderator, usr, reason, create_date, context) values(?, ?, ?, ?, ?)",
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

        Ok(Ban {
            id,
            moderator,
            user,
            reason,
            create_date,
            context,
        })
    }

    pub async fn get_bans(&self, user: UserId) -> Result<Vec<Ban>> {
        let mut conn = self.pool.acquire().await?;
        let id = user.0 as i64;
        Ok(sqlx::query!(
            r#"select id, moderator, usr, reason as "reason!", create_date as "create_date!", context from ban where usr=?"#,
            id
        )
        .fetch_all(&mut conn)
        .await?
        .into_iter()
        .map(|x| Ban {
            id: x.id,
            moderator: UserId(x.moderator as u64),
            user: UserId(x.usr as u64),
            reason: x.reason,
            create_date: chrono::DateTime::from_utc(x.create_date, Utc),
            context: x.context,
        }).collect())
    }
}
