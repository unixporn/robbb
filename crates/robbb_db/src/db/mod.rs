use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{bail, Context, Result};

use serenity::model::id::UserId;
use serenity::prelude::RwLock;
use serenity::prelude::TypeMapKey;
use sqlx::SqlitePool;
use std::collections::HashMap;
pub mod blocklist;
pub mod emoji_logging;
pub mod fetch;
pub mod fetch_field;
pub mod highlights;
pub mod htm;
pub mod mod_action;
pub mod mute;
pub mod tag;

#[derive(Debug)]
pub struct Db {
    pool: SqlitePool,
    blocklist_cache: Arc<RwLock<Option<Vec<String>>>>,
    highlight_cache: RwLock<Option<highlights::HighlightsData>>,
    tag_name_cache: RwLock<Option<HashSet<String>>>,
}

impl TypeMapKey for Db {
    type Value = Arc<Db>;
}

impl Db {
    pub async fn new() -> Result<Self> {
        let pool = SqlitePool::connect(&std::env::var("DATABASE_URL")?).await?;
        Ok(Self {
            pool,
            blocklist_cache: Arc::new(RwLock::new(None)),
            highlight_cache: RwLock::new(None),
            tag_name_cache: RwLock::new(None),
        })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .context("Failed to run database migrations")?;
        Ok(())
    }
}
