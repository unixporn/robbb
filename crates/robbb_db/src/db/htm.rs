use serenity::model::id::UserId;

use crate::Db;

#[derive(Debug)]
pub struct HardToModerateEntry {
    pub user: UserId,
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn check_user_htm(&self, id: UserId) -> anyhow::Result<bool> {
        let id = id.0 as i64;
        Ok(sqlx::query!(r#"select * from hard_to_moderate where usr=?"#, id)
            .fetch_optional(&self.pool)
            .await?
            .is_some())
    }

    #[tracing::instrument(skip_all)]
    pub async fn add_htm(&self, id: UserId) -> anyhow::Result<()> {
        let id = id.0 as i64;
        sqlx::query!(r#"insert or ignore into hard_to_moderate (usr) values (?)"#, id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn remove_htm(&self, id: UserId) -> anyhow::Result<()> {
        let id = id.0 as i64;
        sqlx::query!(r#"delete from hard_to_moderate where usr=?"#, id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(())
    }
}
