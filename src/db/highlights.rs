use super::*;
use itertools::Itertools;

impl Db {
    pub async fn get_highlights(&self) -> Result<HashMap<String, Vec<UserId>>> {
        let mut cache = self.highlight_cache.lock().await;
        // we have a cache
        if let Some(cache) = cache.as_ref() {
            Ok(cache.clone())
        // we don't have a cache at all
        } else {
            let mut conn = self.pool.acquire().await?;
            let q = sqlx::query!("select * from highlights")
                .fetch_all(&mut conn)
                .await?
                .into_iter()
                .map(|x| (x.word, UserId::from(x.user as u64)))
                .into_group_map();
            cache.replace(q.clone());
            Ok(q)
        }
    }

    pub async fn remove_highlight(&self, user: UserId, word: String) -> Result<()> {
        let mut cache = self.highlight_cache.lock().await;
        let mut conn = self.pool.acquire().await?;
        let user = user.0 as i64;
        sqlx::query!("delete from highlights where word=? and user=?", word, user)
            .execute(&mut conn)
            .await?;
        if let Some(ref mut cache) = cache.as_mut() {
            if cache.remove(&word).is_none() {
                bail!("Can't remove a highlight that doesn't exist.")
            }
        }
        Ok(())
    }

    pub async fn set_highlight(&self, user: UserId, word: String) -> Result<()> {
        let mut cache = self.highlight_cache.lock().await;
        let mut conn = self.pool.acquire().await?;
        let user = user.0 as i64;
        sqlx::query!(
            "insert into highlights (word, user) values (?, ?)",
            word,
            user
        )
        .execute(&mut conn)
        .await?;
        let user = user as u64;
        // if we have a cache, modify it
        if let Some(ref mut cache) = cache.as_mut() {
            cache
                .entry(word)
                .and_modify(|f| f.push(UserId::from(user)))
                .or_insert_with(|| vec![UserId::from(user)]);
        }
        Ok(())
    }
}
