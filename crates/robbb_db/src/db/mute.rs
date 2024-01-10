use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct Mute {
    pub id: i64,
    pub moderator: UserId,
    pub user: UserId,
    pub reason: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub context: Option<String>,
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn get_newly_expired_mutes(&self) -> Result<Vec<Mute>> {
        sqlx::query!(
            "SELECT * from mute, mod_action
             WHERE mute.mod_action = mod_action.id
               AND cast(strftime('%s', end_time) as integer) < cast(strftime('%s', datetime('now')) as integer)
               AND active"
        )
        .fetch_all(&self.pool).await?
        .into_iter()
        .map(|x| Ok(Mute {
            id: x.id,
            moderator: UserId::new(x.moderator as u64),
            user: UserId::new(x.usr as u64),
            reason: x.reason.unwrap_or_default(),
            start_time: DateTime::<Utc>::from_naive_utc_and_offset(x.create_date.context("no create date")?, Utc),
            end_time: DateTime::<Utc>::from_naive_utc_and_offset(x.end_time, Utc),
            context: x.context,
        }))
        .collect::<Result<_>>()
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_mutes(&self, user_id: UserId) -> Result<Vec<Mute>> {
        let id: i64 = user_id.into();
        sqlx::query!(
            "select * from mute, mod_action where mute.mod_action = mod_action.id AND usr=?",
            id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|x| {
            Ok(Mute {
                id: x.id,
                moderator: UserId::new(x.moderator as u64),
                user: UserId::new(x.usr as u64),
                reason: x.reason.unwrap_or_default(),
                start_time: DateTime::<Utc>::from_naive_utc_and_offset(
                    x.create_date.context("no create date")?,
                    Utc,
                ),
                end_time: DateTime::<Utc>::from_naive_utc_and_offset(x.end_time, Utc),
                context: x.context,
            })
        })
        .collect::<Result<_>>()
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_active_mute(&self, user_id: UserId) -> Result<Option<Mute>> {
        let id: i64 = user_id.into();
        sqlx::query!("select * from mute, mod_action where mute.mod_action = mod_action.id AND usr=? AND active=true", id)
        .fetch_optional(&self.pool)
        .await?
        .map(|x| Ok(Mute {
            id: x.id,
            moderator: UserId::new(x.moderator as u64),
            user: UserId::new(x.usr as u64),
            reason: x.reason.unwrap_or_default(),
            start_time: DateTime::<Utc>::from_naive_utc_and_offset(x.create_date.context("no create date")?, Utc),
            end_time: DateTime::<Utc>::from_naive_utc_and_offset(x.end_time, Utc),
            context: x.context,
        }))
        .transpose()
    }

    #[tracing::instrument(skip_all)]
    pub async fn remove_active_mutes(&self, user_id: UserId) -> Result<()> {
        let id: i64 = user_id.into();
        sqlx::query!(
            "update mute set active=false
            from mute m
            join mod_action on mod_action.id = m.mod_action
            where mod_action.usr=? and m.active=true
            ",
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn set_mute_inactive(&self, id: i64) -> Result<()> {
        sqlx::query!("update mute set active = false where mod_action = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
