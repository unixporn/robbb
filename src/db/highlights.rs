use super::Db;
use anyhow::*;
use serenity::model::id::UserId;
use std::collections::HashMap;

impl Db {
    pub async fn get_highlights(&self) -> Result<HashMap<String, Vec<UserId>>> {
        let mut conn = self.pool.acquire().await?;
        let mut map: HashMap<String, Vec<UserId>> = HashMap::new();
        let q = sqlx::query!("select * from highlights")
            .fetch_all(&mut conn)
            .await?
            .into_iter()
            .map(|x| (x.word, x.user));
        for i in q {
            if map.get(i.1).is_some() {
                map.get(i.1).unwrap().push(UserId::from(i.2));
            } else {
                map.insert(i.1, vec![i.2]);
            }
        }
        Ok(map)
    }
    pub async fn get_higlights_user(&self, id: UserId) -> Vec<String> {}
    pub async fn set_highlight(&self, user: UserId, word: String) -> Result<()> {
        Ok(())
    }
}
