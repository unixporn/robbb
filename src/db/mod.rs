use std::sync::Arc;

use anyhow::*;
use chrono::{DateTime, Utc};
use serenity::{model::id::UserId, prelude::TypeMapKey};
use sqlx::{prelude::*, types::Uuid, SqlitePool};
use warn::Warn;

pub mod mute;
pub mod warn;

pub struct Db {
    pool: SqlitePool,
}

impl TypeMapKey for Db {
    type Value = Arc<Db>;
}

impl Db {
    pub async fn new() -> Result<Self> {
        let pool = SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?;
        Ok(Self { pool })
    }
}
