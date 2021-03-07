use anyhow::*;
use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct Profile {
    pub user: UserId,
    pub git: Option<String>,
    pub dotfiles: Option<String>,
    pub description: Option<String>,
}

impl Db {
    pub async fn set_git(&self, user: UserId, value: Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let user = user.0 as i64;

        // TODO handling of insert vs. update definitely belongs into sql, not like this
        let result = sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?, ?, NULL, NULL)",
            user,
            value
        )
        .execute(&mut conn)
        .await;
        if result.is_err() {
            sqlx::query!("update profile set git=? where usr=?", user, value)
                .execute(&mut conn)
                .await?;
        }
        Ok(())
    }

    pub async fn set_dotfiles(&self, user: UserId, value: Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let user = user.0 as i64;
        let result = sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?, NULL, ?, NULl)",
            user,
            value
        )
        .execute(&mut conn)
        .await;

        if result.is_err() {
            sqlx::query!("update profile set dotfiles=? where usr=?", user, value)
                .execute(&mut conn)
                .await?;
        }
        Ok(())
    }

    pub async fn set_description(&self, user: UserId, value: Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let user = user.0 as i64;
        let result = sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?, NULL, NULL, ?)",
            user,
            value
        )
        .execute(&mut conn)
        .await;
        if result.is_err() {
            sqlx::query!("update profile set description=? where usr=?", user, value)
                .execute(&mut conn)
                .await?;
        }
        Ok(())
    }

    pub async fn get_profile(&self, user_id: UserId) -> Result<Profile> {
        let mut conn = self.pool.acquire().await?;

        let user = user_id.0 as i64;
        Ok(sqlx::query!("select * from profile where usr=?", user)
            .fetch_optional(&mut conn)
            .await?
            .map(|x| Profile {
                user: UserId(x.usr as u64),
                description: x.description,
                git: x.git,
                dotfiles: x.dotfiles,
            })
            .unwrap_or_else(|| Profile {
                user: user_id,
                description: None,
                git: None,
                dotfiles: None,
            }))
    }
}
