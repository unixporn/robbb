use std::collections::HashMap;

use anyhow::*;

use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct Fetch {
    pub user: UserId,
    pub info: HashMap<String, String>,
}

impl Fetch {
    pub fn get_values_ordered(mut self) -> Vec<(String, String)> {
        let mut entries: Vec<(String, String)> = crate::commands::fetch::NORMAL_FETCH_KEYS
            .iter()
            .filter_map(|x| Some((x.to_string(), self.info.remove(*x)?)))
            .collect();
        if let Some(image) = self.info.remove(crate::commands::fetch::IMAGE_KEY) {
            entries.push((crate::commands::fetch::IMAGE_KEY.to_string(), image));
        }
        entries
    }
}

impl Db {
    pub async fn set_fetch(&self, user: UserId, info: HashMap<String, String>) -> Result<Fetch> {
        let mut conn = self.pool.acquire().await?;
        {
            let user = user.0 as i64;
            let info = serde_json::to_string(&info)?;

            sqlx::query!(
                "insert into fetch (usr, info) values (?1, ?2) on conflict(usr) do update set info=?2",
                user,
                info,
            )
            .execute(&mut conn)
            .await?;
        }

        Ok(Fetch { user, info })
    }

    pub async fn get_fetch(&self, user: UserId) -> Result<Option<Fetch>> {
        let mut conn = self.pool.acquire().await?;
        let user = user.0 as i64;
        let value = sqlx::query!("select * from fetch where usr=?", user)
            .fetch_optional(&mut conn)
            .await?;
        if let Some(x) = value {
            Ok(Some(Fetch {
                user: UserId(x.usr as u64),
                info: serde_json::from_str(&x.info).context("Failed to deserialize fetch data")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_fetch(
        &self,
        user: UserId,
        new_values: HashMap<String, String>,
    ) -> Result<Fetch> {
        let mut fetch = self
            .get_fetch(user)
            .await?
            .map(|x| x.info)
            .unwrap_or_default();

        for (key, value) in new_values {
            fetch.insert(key, value);
        }

        Ok(self.set_fetch(user, fetch).await?)
    }

    pub async fn get_all_fetches(&self) -> Result<Vec<Fetch>> {
        let mut conn = self.pool.acquire().await?;
        Ok(sqlx::query!("select * from fetch")
            .fetch_all(&mut conn)
            .await?
            .into_iter()
            .map(|x| {
                Ok(Fetch {
                    user: UserId(x.usr as u64),
                    info: serde_json::from_str(&x.info)
                        .context("Failed to deserialize fetch data")?,
                })
            })
            .collect::<Result<_>>()?)
    }
}
