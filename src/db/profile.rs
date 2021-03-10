use anyhow::*;
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
        sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?, ?, NULL, NULL)
                on conflict(usr) do update set git=?",
            user,
            value,
            value
        )
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    pub async fn set_dotfiles(&self, user: UserId, value: Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let user = user.0 as i64;
        sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?, NULL, ?, NULl)
                on conflict(usr) do update set dotfiles=?",
            user,
            value,
            value
        )
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    pub async fn set_description(&self, user: UserId, value: Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let user = user.0 as i64;
        sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?, NULL, NULL, ?)
                on conflict(usr) do update set description=?",
            user,
            value,
            value
        )
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    pub async fn get_profile(&self, user_id: UserId) -> Result<Option<Profile>> {
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
            }))
    }
}
