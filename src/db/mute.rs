use anyhow::Result;
use chrono::{DateTime, Utc};
use serenity::model::id::{GuildId, UserId};

use super::Db;

#[derive(Debug)]
pub struct Mute {
    pub id: i64,
    pub guild_id: GuildId,
    pub moderator: UserId,
    pub user: UserId,
    pub reason: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub context: Option<String>,
}

impl Db {
    // I did not sign up for this @elkowar, you fix
    #[allow(clippy::too_many_arguments)]
    pub async fn add_mute(
        &self,
        guild_id: GuildId,
        moderator: UserId,
        user: UserId,
        reason: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        context: Option<String>,
    ) -> Result<Mute> {
        let mut conn = self.pool.acquire().await?;

        let id = {
            let guild_id = guild_id.0 as i64;
            let moderator = moderator.0 as i64;
            let user = user.0 as i64;
            sqlx::query!(
                "insert into mute (guildid, moderator, usr, reason, start_time, end_time, active, context) values(?, ?, ?, ?, ?, ?, true, ?)",
                guild_id,
                moderator,
                user,
                reason,
                start_time,
                end_time,
                context,
            )
            .execute(&mut conn)
            .await?
            .last_insert_rowid()
        };

        Ok(Mute {
            id,
            guild_id,
            moderator,
            user,
            reason,
            start_time,
            end_time,
            context,
        })
    }

    pub async fn get_newly_expired_mutes(&self) -> Result<Vec<Mute>> {
        let mut conn = self.pool.acquire().await?;
        Ok(sqlx::query!(
            "select * from mute 
            where cast(strftime('%s', end_time) as integer) < cast(strftime('%s', datetime('now')) as integer)
              and active"
        )
        .fetch_all(&mut conn).await?
        .into_iter()
        .map(|x| {Mute {
            id: x.id,
            guild_id: GuildId(x.guildid as u64),
            moderator: UserId(x.moderator as u64),
            user: UserId(x.usr as u64),
            reason: x.reason.unwrap_or_default(),
            start_time: chrono::DateTime::<Utc>::from_utc(x.start_time, Utc),
            end_time: chrono::DateTime::<Utc>::from_utc(x.end_time, Utc),
            context: x.context,
        }})
        .collect())
    }

    pub async fn get_mutes(&self, user_id: UserId) -> Result<Vec<Mute>> {
        let mut conn = self.pool.acquire().await?;
        let id = user_id.0 as i64;
        Ok(sqlx::query!("select * from mute where usr=?", id)
            .fetch_all(&mut conn)
            .await?
            .into_iter()
            .map(|x| Mute {
                id: x.id,
                guild_id: GuildId(x.guildid as u64),
                moderator: UserId(x.moderator as u64),
                user: UserId(x.usr as u64),
                reason: x.reason.unwrap_or_default(),
                start_time: chrono::DateTime::<Utc>::from_utc(x.start_time, Utc),
                end_time: chrono::DateTime::<Utc>::from_utc(x.end_time, Utc),
                context: x.context,
            })
            .collect())
    }

    pub async fn remove_active_mutes(&self, user_id: UserId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let id = user_id.0 as i64;
        sqlx::query!(
            "update mute set active=false where usr=? and active=true",
            id
        )
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    pub async fn set_mute_inactive(&self, id: i64) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!("update mute set active = false where id = ?", id)
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}
