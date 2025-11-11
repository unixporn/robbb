use serenity::model::id::UserId;

use crate::Db;

#[derive(Debug)]
pub struct HardToModerateEntry {
    pub user: UserId,
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn check_user_htm(&self, id: UserId) -> eyre::Result<bool> {
        let id: i64 = id.into();
        Ok(sqlx::query!(r#"select * from hard_to_moderate where usr=?"#, id)
            .fetch_optional(&self.pool)
            .await?
            .is_some())
    }

    #[tracing::instrument(skip_all)]
    pub async fn add_htm(&self, id: UserId) -> eyre::Result<()> {
        let id: i64 = id.into();
        sqlx::query!(r#"insert or ignore into hard_to_moderate (usr) values (?)"#, id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn remove_htm(&self, id: UserId) -> eyre::Result<()> {
        let id: i64 = id.into();
        sqlx::query!(r#"delete from hard_to_moderate where usr=?"#, id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(())
    }
}
