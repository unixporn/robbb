use super::*;
use anyhow::*;

impl Db {
    pub async fn get_highlights(&self) -> Result<HashMap<String, Vec<UserId>>> {
        let mut cache = &self.highlight_cache.lock().await;
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
                .map(|x| (x.word, x.user));
            for i in q {
                let mut cache = cache.as_ref();
                cache = Some(&HashMap::new());
                // checks if a highlight with that string already exists,
                // if it does it appends the user to the lift of to be pinged users.
                if cache.unwrap().get(i.1).is_some() {
                    cache.unwrap().get(i.1).unwrap().push(UserId::from(i.2));
                } else {
                    cache.unwrap().insert(i.1, vec![i.2]);
                }
            }
            return Ok(cache.as_ref().clone().unwrap().clone());
        }
    }
    /*
    pub async fn get_higlights_user(&self, id: UserId) -> Result<Vec<String>> {
        let conn = self.pool.acquire().await?;
        Ok(sqlx::query!("select * from highlights where user=?", id)
            .fetch_all(&mut conn)
            .await?
            .into_iter()
            .map(|x| (x.word, x.user))
            .collect::<Vec<String>>())
    }*/
    pub async fn set_highlight(&self, user: UserId, word: String) -> Result<()> {
        let cache = &self.highlight_cache.lock().await;
        let conn = self.pool.acquire().await?;
        sqlx::query!(
            "insert into highlights (word, user) values (?, ?)",
            word,
            user.1
        )?;
        if let Some(cache) = cache.as_ref() {
            if let Some(a) = cache.get(&word) {
                a.push(UserId::from(user));
            } else {
                cache.insert(word, vec![user]);
            }
        }
        Ok(())
    }
}
