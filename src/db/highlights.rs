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
                .map(|x| (x.word, UserId::from(x.usr as u64)))
                .into_group_map();
            cache.replace(q.clone());
            Ok(q)
        }
    }

    pub async fn remove_highlight(&self, user: UserId, word: String) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let user = user.0 as i64;
        sqlx::query!("delete from highlights where word=? and usr=?", word, user)
            .execute(&mut conn)
            .await?;
        let mut cache = self.highlight_cache.lock().await;
        if let Some(ref mut cache) = cache.as_mut() {
            if let std::collections::hash_map::Entry::Occupied(ref mut entry) =
                cache.entry(word.clone())
            {
                let array = entry.get_mut();
                array.retain(|u| u.0 as usize != user as usize);
                if array.is_empty() {
                    cache.remove(&word);
                }
            } else {
                bail!("Can't remove a highlight that doesn't exist.")
            }
        }
        Ok(())
    }

    pub async fn set_highlight(&self, user: UserId, word: String) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let user = user.0 as i64;
        sqlx::query!(
            "insert into highlights (word, usr) values (?, ?)",
            word,
            user
        )
        .execute(&mut conn)
        .await?;
        let user = user as u64;
        let mut cache = self.highlight_cache.lock().await;
        // if we have a cache, modify it
        if let Some(ref mut cache) = cache.as_mut() {
            cache
                .entry(word)
                .and_modify(|f| {
                    let id = UserId::from(user);
                    if !f.contains(&id) {
                        f.push(id);
                    } // we can't return a error from within a closure, so we'll just have to tell the user that everything was ok,
                      // after all they still get a notif, so they're command ran sucessfully.
                })
                .or_insert_with(|| vec![UserId::from(user)]);
        }
        Ok(())
    }

    pub async fn rm_highlights_of(&self, user: UserId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let useri64 = user.0 as i64;
        sqlx::query!("delete from highlights where usr=?", useri64)
            .execute(&mut conn)
            .await?;

        let mut cache = self.highlight_cache.lock().await;
        // checks if we have a cache
        if let Some(ref mut cache) = cache.as_mut() {
            // iterates over every where word where the user is subscribed to
            for i in cache
                .clone()
                .iter()
                .filter(|(_, users)| users.contains(&user))
            {
                if let std::collections::hash_map::Entry::Occupied(ref mut entry) =
                    cache.entry(i.0.clone())
                {
                    // removes it
                    let array = entry.get_mut();
                    array.retain(|u| u.0 as usize != useri64 as usize);
                    if array.is_empty() {
                        cache.remove(i.0);
                    }
                }
            }
        }
        Ok(())
    }
}
