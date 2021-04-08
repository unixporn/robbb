use std::sync::Arc;

use anyhow::*;

use serenity::{futures::lock::Mutex, prelude::TypeMapKey};
use sqlx::SqlitePool;

pub mod blocklist;
pub mod emoji_logging;
pub mod fetch;
pub mod mute;
pub mod note;
pub mod profile;
pub mod tag;
pub mod warn;

pub struct Db {
    pool: SqlitePool,
    blocklist_cache: Arc<Mutex<Option<Vec<String>>>>,
}

impl TypeMapKey for Db {
    type Value = Arc<Db>;
}

impl Db {
    pub async fn new() -> Result<Self> {
        let pool = SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?;
        Ok(Self {
            pool,
            blocklist_cache: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run database migrations")?;
        Ok(())
    }
}
