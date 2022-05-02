use std::collections::HashMap;

use anyhow::{Context, Result};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::model::id::UserId;

use super::Db;

// TODORW this and fetch field belong into fetch.rs in commands, not here
pub static FETCH_KEY_ORDER: [FetchField; 14] = [
    FetchField::Distro,
    FetchField::Kernel,
    FetchField::Terminal,
    FetchField::Editor,
    FetchField::DEWM,
    FetchField::Bar,
    FetchField::Resolution,
    FetchField::DisplayProtocol,
    FetchField::Shell,
    FetchField::GTK3,
    FetchField::Icons,
    FetchField::CPU,
    FetchField::GPU,
    FetchField::Memory,
];

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub enum FetchField {
    Distro,
    Kernel,
    Terminal,
    Editor,
    #[serde(rename = "DE/WM")]
    DEWM,
    Bar,
    Resolution,
    #[serde(rename = "Display Protocol")]
    DisplayProtocol,
    Shell,
    #[serde(rename = "GTK3 Theme")]
    GTK3,
    #[serde(rename = "GTK Icon Theme")]
    Icons,
    CPU,
    GPU,
    Memory,
    #[serde(rename = "image")]
    Image,
}

impl std::fmt::Display for FetchField {
    fn fmt(&self, writer: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FetchField::DEWM => write!(writer, "DE/WM"),
            FetchField::DisplayProtocol => write!(writer, "Display Protocol"),
            FetchField::GTK3 => write!(writer, "GTK3 Theme"),
            FetchField::Icons => write!(writer, "GTK Icon Theme"),
            FetchField::Image => write!(writer, "image"),
            _ => write!(writer, "{:?}", self),
        }
    }
}

impl std::str::FromStr for FetchField {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "distro" => Ok(Self::Distro),
            "kernel" => Ok(Self::Kernel),
            "terminal" => Ok(Self::Terminal),
            "editor" => Ok(Self::Editor),
            "dewm" | "de" | "wm" | "de/wm" => Ok(Self::DEWM),
            "bar" => Ok(Self::Bar),
            "resolution" => Ok(Self::Resolution),
            "display protocol" => Ok(Self::DisplayProtocol),
            "shell" => Ok(Self::Shell),
            "gtk theme" | "gtk3 theme" | "theme" | "gtk" => Ok(Self::GTK3),
            "icons" | "icon theme" | "gtk icon theme" => Ok(Self::Icons),
            "cpu" => Ok(Self::CPU),
            "gpu" => Ok(Self::GPU),
            "memory" => Ok(Self::Memory),
            "image" => Ok(Self::Image),
            _ => Err("Not a valid fetch field.".into()),
        }
    }
}

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

        self.set_fetch(user, fetch, Some(Utc::now())).await
    }

    pub async fn get_all_fetches(&self) -> Result<Vec<Fetch>> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!("select * from fetch")
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
            .collect::<Result<_>>()
    }
}
