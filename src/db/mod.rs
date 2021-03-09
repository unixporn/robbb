use std::sync::Arc;

use anyhow::*;

use serenity::prelude::TypeMapKey;
use sqlx::SqlitePool;

pub mod fetch;
pub mod mute;
pub mod note;
pub mod profile;
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
