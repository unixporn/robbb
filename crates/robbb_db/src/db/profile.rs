use std::collections::HashMap;

use anyhow::Result;
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct Profile {
    pub user: UserId,
    pub git: Option<String>,
    pub dotfiles: Option<String>,
    pub description: Option<String>,
}

impl Profile {
    pub fn into_values_map(self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        if let Some(description) = self.description {
            m.insert("description".to_string(), description);
        }
        if let Some(dotfiles) = self.dotfiles {
            m.insert("dotfiles".to_string(), dotfiles);
        }
        if let Some(git) = self.git {
            m.insert("git".to_string(), git);
        }
        m
    }
}

impl Db {
    pub async fn set_git(&self, user: UserId, value: Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let user = user.0 as i64;
        sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?1, ?2, NULL, NULL)
                on conflict(usr) do update set git=?2",
            user,
            value,
        )
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    pub async fn set_dotfiles(&self, user: UserId, value: Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let user = user.0 as i64;
        sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?1, NULL, ?2, NULl)
                on conflict(usr) do update set dotfiles=?2",
            user,
            value,
        )
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    pub async fn set_description(&self, user: UserId, value: Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;

        let user = user.0 as i64;
        sqlx::query!(
            "insert into profile (usr, git, dotfiles, description) values (?1, NULL, NULL, ?2)
                on conflict(usr) do update set description=?2",
            user,
            value,
        )
        .execute(&mut conn)
        .await?;
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
