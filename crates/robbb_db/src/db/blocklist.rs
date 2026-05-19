use std::sync::Arc;

use eyre::Result;
use regex::{Regex, RegexBuilder};
use serenity::model::id::UserId;

use super::Db;

/// Test strings that the combined blocklist regex must never match. If any pattern
/// in the blocklist matches one of these (e.g. someone accidentally adds an empty
/// pattern), we disable the blocklist instead of nuking innocent messages.
pub static SHOULD_NEVER_TRIGGER_BLOCKLIST: &[&str] = &[
    "",
    "Hello, I am new to linux, and I'd love to get some help with my GNOME installation.",
    "I use Arch with GNOME, but for some reason, my backspace key doesn't work properly. Someone please help",
];

#[derive(Debug)]
pub struct BlocklistData {
    pub patterns: Vec<String>,
    pub regex: Regex,
}

impl BlocklistData {
    fn build(patterns: Vec<String>) -> Result<Self> {
        let regex = if patterns.is_empty() {
            never_matching_regex()
        } else {
            RegexBuilder::new(&patterns.join("|")).case_insensitive(true).build()?
        };

        if SHOULD_NEVER_TRIGGER_BLOCKLIST.iter().any(|x| regex.is_match(x)) {
            tracing::error!(
                "Blocklist regex matches one of the sanity check patterns. Disabling blocklist until it is fixed."
            );
            return Ok(BlocklistData { patterns, regex: never_matching_regex() });
        }

        Ok(BlocklistData { patterns, regex })
    }
}

fn never_matching_regex() -> Regex {
    Regex::new("a^").unwrap()
}

impl Db {
    /// Returns the cached blocklist, including the compiled combined regex.
    /// Cheap to call: the data is wrapped in an `Arc` and shared across calls.
    pub async fn get_blocklist(&self) -> Result<Arc<BlocklistData>> {
        {
            let cache = self.blocklist_cache.read().await;
            if let Some(data) = cache.as_ref() {
                return Ok(data.clone());
            }
        }

        let mut cache = self.blocklist_cache.write().await;
        if let Some(data) = cache.as_ref() {
            return Ok(data.clone());
        }
        let patterns = sqlx::query_scalar!(r#"select pattern as "pattern!" from blocked_regexes"#)
            .fetch_all(&self.pool)
            .await?;
        let data = Arc::new(BlocklistData::build(patterns)?);
        *cache = Some(data.clone());
        Ok(data)
    }

    pub async fn add_blocklist_entry(&self, user_id: UserId, s: &str) -> Result<()> {
        let user_id: i64 = user_id.into();
        sqlx::query!("insert into blocked_regexes(pattern, added_by) values (?, ?)", s, user_id)
            .execute(&self.pool)
            .await?;

        // Invalidate the cache so the next reader rebuilds it (rare op).
        *self.blocklist_cache.write().await = None;
        Ok(())
    }

    pub async fn remove_blocklist_entry(&self, s: &str) -> Result<()> {
        sqlx::query!("delete from blocked_regexes where pattern=?", s).execute(&self.pool).await?;
        *self.blocklist_cache.write().await = None;
        Ok(())
    }
}

