use anyhow::*;
use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct Warn {
    pub id: i64,
    pub moderator: UserId,
    pub user: UserId,
    pub reason: String,
    pub create_date: DateTime<Utc>,
}

impl Db {
    pub async fn add_warn(
        &self,
        moderator: UserId,
        user: UserId,
        reason: String,
        create_date: DateTime<Utc>,
    ) -> Result<Warn> {
        let mut conn = self.pool.acquire().await?;

        let id = {
            let moderator = moderator.0 as i64;
            let user = user.0 as i64;
            sqlx::query!(
                "insert into warn (moderator, usr, reason, create_date) values(?, ?, ?, ?)",
                moderator,
                user,
                reason,
                create_date,
            )
            .execute(&mut conn)
            .await?
            .last_insert_rowid()
        };

        Ok(Warn {
            id,
            moderator,
            user,
            reason,
            create_date,
        })
    }

    pub async fn count_warns(&self, user: UserId) -> Result<i32> {
        let mut conn = self.pool.acquire().await?;
        let id = user.0 as i64;
        Ok(
            sqlx::query_scalar!("select count(*) from warn where usr=?", id)
                .fetch_one(&mut conn)
                .await?,
        )
    }
}
