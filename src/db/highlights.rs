use super::*;


impl Db {
    pub async fn get_highlights(&self) -> Result<HashMap<String, Vec<UserId>>> {
        let mut cache = self.highlight_cache.lock().await;
        // we have a cache
        if let Some(cache) = cache.as_ref() {
            return Ok(cache.clone());
        // we don't have a cache at all
        } else {
            let mut conn = self.pool.acquire().await?;
            let q = sqlx::query!("select * from highlights")
                .fetch_all(&mut conn)
                .await?
                .into_iter()
                .map(|x| (x.word, x.user))
                .collect::<Vec<(String, i64)>>();
            let map = gen_map(q).await;
            cache.replace(map.clone());
            return Ok(map);
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
        if let Some(ref mut cache) = cache.as_mut() {
            if cache.get(&word).is_some() {
                cache.entry(word).and_modify(|f| f.push(UserId::from(user)));
            } else {
                cache.insert(word, vec![UserId::from(user)]);
            }
        }
        Ok(())
    }
}

async fn gen_map(iter: Vec<(String, i64)>) -> HashMap<String, Vec<UserId>> {
    let mut cache: HashMap<String, Vec<UserId>> = HashMap::new();
    for i in iter {
        // checks if a highlight with that string already exists,
        // if it does it appends the user to the lift of to be pinged users.
        if cache.get(&i.0).is_some() {
            cache
                .entry(i.clone().0)
                .and_modify(|f| f.push(UserId::from(i.clone().1 as u64)));
        } else {
            cache.insert(i.0, vec![UserId::from(i.1 as u64)]);
        }
    }
    return cache.clone();
}
