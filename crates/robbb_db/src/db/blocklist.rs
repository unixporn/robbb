use anyhow::Result;
use itertools::Itertools;
use regex::{Regex, RegexBuilder};
use serenity::model::id::UserId;

use super::Db;

impl Db {
    pub async fn get_combined_blocklist_regex(&self) -> Result<Regex> {
        let blocklist = self.get_blocklist().await?;
        if blocklist.is_empty() {
            Ok(Regex::new("a^").unwrap())
        } else {
            Ok(RegexBuilder::new(&blocklist.join("|")).case_insensitive(true).build()?)
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_blocklist(&self) -> Result<Vec<String>> {
        let mut conn = self.pool.acquire().await?;

        let mut cache = self.blocklist_cache.write().await;

        if let Some(cache) = cache.as_ref() {
            Ok(cache.clone())
        } else {
            let rows = sqlx::query_scalar!(r#"select pattern as "pattern!" from blocked_regexes"#)
                .fetch_all(&mut conn)
                .await?;
            *cache = Some(rows.clone());
            Ok(rows)
        }
    }

    pub async fn add_blocklist_entry(&self, user_id: UserId, s: &str) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let user_id = user_id.0 as i64;
        sqlx::query!("insert into blocked_regexes(pattern, added_by) values (?, ?)", s, user_id)
            .execute(&mut conn)
            .await?;

        let mut cache = self.blocklist_cache.write().await;
        if let Some(ref mut cache) = cache.as_mut() {
            cache.push(s.to_string());
        }

        Ok(())
    }

    pub async fn remove_blocklist_entry(&self, s: &str) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!("delete from blocked_regexes where pattern=?", s).execute(&mut conn).await?;

        let mut cache = self.blocklist_cache.write().await;
        if let Some(ref mut cache) = cache.as_mut() {
            if let Some((pos, _)) = cache.iter().find_position(|x| x.as_str() == s) {
                cache.remove(pos);
            }
        }

        Ok(())
    }
}
