use std::collections::HashMap;

use anyhow::{Context, Result};

use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

use crate::commands::fetch::{FetchField, FETCH_KEY_ORDER};

#[derive(Debug)]
pub struct Fetch {
    pub user: UserId,
    pub info: HashMap<FetchField, String>,
    pub create_date: Option<DateTime<Utc>>,
}

impl Fetch {
    pub fn get_values_ordered(mut self) -> Vec<(FetchField, String)> {
        let mut entries: Vec<(FetchField, String)> = FETCH_KEY_ORDER
            .iter()
            .filter_map(|x| Some((x.clone(), self.info.remove(x)?)))
            .collect();
        if let Some(image) = self.info.remove(&FetchField::Image) {
            entries.push((FetchField::Image, image));
        }
        entries
    }
}

impl Db {
    pub async fn set_fetch(
        &self,
        user: UserId,
        info: HashMap<FetchField, String>,
        create_date: Option<DateTime<Utc>>,
    ) -> Result<Fetch> {
        let mut conn = self.pool.acquire().await?;
        {
            let user = user.0 as i64;
            let info = serde_json::to_string(&info)?;

            sqlx::query!(
                "insert into fetch (usr, info, create_date) values (?1, ?2, ?3) on conflict(usr) do update set info=?2, create_date=?3",
                user,
                info,
                create_date,
            )
            .execute(&mut conn)
            .await?;
        }

        Ok(Fetch {
            user,
            info,
            create_date,
        })
    }

    pub async fn get_fetch(&self, user: UserId) -> Result<Option<Fetch>> {
        let mut conn = self.pool.acquire().await?;
        let user = user.0 as i64;
        let value = sqlx::query!("select * from fetch where usr=?", user)
            .fetch_optional(&mut conn)
            .await?;
        if let Some(x) = value {
            let create_date = x
                .create_date
                .map(|date| chrono::DateTime::from_utc(date, chrono::Utc));
            Ok(Some(Fetch {
                user: UserId(x.usr as u64),
                info: serde_json::from_str(&x.info).context("Failed to deserialize fetch data")?,
                create_date,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_fetch(
        &self,
        user: UserId,
        new_values: HashMap<FetchField, String>,
    ) -> Result<Fetch> {
        let mut fetch = self
            .get_fetch(user)
            .await?
            .map(|x| x.info)
            .unwrap_or_default();

        for (key, value) in new_values {
            fetch.insert(key, value);
        }

        Ok(self.set_fetch(user, fetch, Some(Utc::now())).await?)
    }

    pub async fn get_all_fetches(&self) -> Result<Vec<Fetch>> {
        let mut conn = self.pool.acquire().await?;
        Ok(sqlx::query!("select * from fetch")
            .fetch_all(&mut conn)
            .await?
            .into_iter()
            .map(|x| {
                let create_date = x
                    .create_date
                    .map(|date| chrono::DateTime::from_utc(date, chrono::Utc));
                Ok(Fetch {
                    user: UserId(x.usr as u64),
                    info: serde_json::from_str(&x.info)
                        .context("Failed to deserialize fetch data")?,
                    create_date,
                })
            })
            .collect::<Result<_>>()?)
    }
}
