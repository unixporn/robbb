use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug)]
pub struct ModAction {
    pub id: i64,
    pub moderator: UserId,
    pub user: UserId,
    pub reason: String,
    pub create_date: Option<DateTime<Utc>>,
    pub context: Option<String>,
    pub kind: ModActionKind,
}

#[derive(Debug)]
pub enum ModActionKind {
    ManualNote,
    BlocklistViolation,
    Warn,
    Mute { end_time: DateTime<Utc>, active: bool },
    Ban,
    Kick,
}

impl ModActionKind {
    pub fn to_action_type(&self) -> ModActionType {
        match self {
            ModActionKind::ManualNote => ModActionType::ManualNote,
            ModActionKind::BlocklistViolation => ModActionType::BlocklistViolation,
            ModActionKind::Warn => ModActionType::Warn,
            ModActionKind::Mute { .. } => ModActionType::Mute,
            ModActionKind::Ban => ModActionType::Ban,
            ModActionKind::Kick => ModActionType::Kick,
        }
    }
}

#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash, poise::ChoiceParameter)]
pub enum ModActionType {
    #[name = "Moderator Note"]
    ManualNote,
    #[name = "[AUTO] - Blocklist Violation"]
    BlocklistViolation,
    #[name = "Warning"]
    Warn,
    #[name = "Mute"]
    Mute,
    #[name = "Ban"]
    Ban,
    #[name = "Kick"]
    Kick,
}

impl std::fmt::Display for ModActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModActionType::ManualNote => write!(f, "Moderator Note"),
            ModActionType::BlocklistViolation => write!(f, "Blocklist Violation"),
            ModActionType::Warn => write!(f, "Warning"),
            ModActionType::Mute => write!(f, "Mute"),
            ModActionType::Ban => write!(f, "Ban"),
            ModActionType::Kick => write!(f, "Kick"),
        }
    }
}
impl ModActionType {
    pub fn from_i32(n: i32) -> Result<Self> {
        match n {
            0 => Ok(ModActionType::ManualNote),
            1 => Ok(ModActionType::BlocklistViolation),
            2 => Ok(ModActionType::Warn),
            3 => Ok(ModActionType::Mute),
            4 => Ok(ModActionType::Ban),
            5 => Ok(ModActionType::Kick),
            _ => anyhow::bail!("Invalid mod action type: {}", n),
        }
    }

    pub fn as_i32(&self) -> i32 {
        match self {
            ModActionType::ManualNote => 0,
            ModActionType::BlocklistViolation => 1,
            ModActionType::Warn => 2,
            ModActionType::Mute => 3,
            ModActionType::Ban => 4,
            ModActionType::Kick => 5,
        }
    }
}

struct DbModActionFields {
    id: i64,
    /// Necessary field because it's part of the query output
    #[allow(unused)]
    mod_action: Option<i64>,
    moderator: i64,
    usr: i64,
    reason: Option<String>,
    create_date: Option<NaiveDateTime>,
    context: Option<String>,
    action_type: i64,
    end_time: Option<NaiveDateTime>,
    active: Option<bool>,
}

impl DbModActionFields {
    fn into_mod_action(self) -> Result<ModAction> {
        Ok(ModAction {
            id: self.id,
            moderator: UserId::new(self.moderator as u64),
            user: UserId::new(self.usr as u64),
            reason: self.reason.unwrap_or_default(),
            create_date: self
                .create_date
                .map(|x| chrono::DateTime::from_naive_utc_and_offset(x, Utc)),
            context: self.context,
            kind: match ModActionType::from_i32(self.action_type as i32)? {
                ModActionType::ManualNote => ModActionKind::ManualNote,
                ModActionType::BlocklistViolation => ModActionKind::BlocklistViolation,
                ModActionType::Warn => ModActionKind::Warn,
                ModActionType::Mute => ModActionKind::Mute {
                    end_time: chrono::DateTime::from_naive_utc_and_offset(
                        self.end_time.context("no mute item for mute in database")?,
                        Utc,
                    ),
                    active: self.active.context("no mute item for mute in database")?,
                },
                ModActionType::Ban => ModActionKind::Ban,
                ModActionType::Kick => ModActionKind::Kick,
            },
        })
    }
}

impl Db {
    #[tracing::instrument(skip_all,
        fields(
            mod_action.moderator = %moderator.get(),
            mod_action.user_id = %user.get(),
            mod_action.reason = %reason,
            mod_action.create_date = %create_date,
            mod_action.context = %context,
            mod_action.kind = ?kind
        )
    )]
    pub async fn add_mod_action(
        &self,
        moderator: UserId,
        user: UserId,
        reason: String,
        create_date: DateTime<Utc>,
        context: String,
        kind: ModActionKind,
    ) -> Result<ModAction> {
        let mut trans = self.pool.begin().await?;

        let id = {
            let moderator: i64 = moderator.into();
            let user: i64 = user.into();
            let action_type = kind.to_action_type().as_i32();
            sqlx::query!(
                "insert into mod_action (moderator, usr, reason, create_date, context, action_type) values(?, ?, ?, ?, ?, ?)",
                moderator,
                user,
                reason,
                create_date,
                context,
                action_type,
            )
            .execute(&mut *trans)
            .await?
            .last_insert_rowid()
        };

        if let ModActionKind::Mute { end_time, active } = kind {
            sqlx::query!(
                "insert into mute (mod_action, end_time, active) VALUES(?, ?, ?)",
                id,
                end_time,
                active
            )
            .execute(&mut *trans)
            .await?;
        }
        trans.commit().await?;

        Ok(ModAction {
            id,
            moderator,
            user,
            reason,
            create_date: Some(create_date),
            context: Some(context),
            kind,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_mod_actions(
        &self,
        user_id: UserId,
        filter: Option<ModActionType>,
    ) -> Result<Vec<ModAction>> {
        let user_id: i64 = user_id.into();

        let note_type_value = filter.map(|x| x.as_i32());

        let mut actions: Vec<ModAction> = sqlx::query_as!(
            DbModActionFields,
            r#"
                SELECT * FROM mod_action
                LEFT JOIN mute ON mod_action.id = mute.mod_action
                WHERE usr=?1 AND (?2 IS NULL OR action_type=?2)
            "#,
            user_id,
            note_type_value,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|x| x.into_mod_action())
        .collect::<Result<Vec<_>>>()?;
        actions.sort_by_key(|x| std::cmp::Reverse(x.create_date));
        Ok(actions)
    }

    #[tracing::instrument(skip_all, fields(mod_action.id = %id))]
    pub async fn get_mod_action(&self, id: i64) -> Result<ModAction> {
        let action = sqlx::query_as!(
            DbModActionFields,
            r#"
                SELECT * FROM mod_action
                LEFT JOIN mute ON mod_action.id = mute.mod_action
                WHERE id=?1
            "#,
            id,
        )
        .fetch_one(&self.pool)
        .await?;
        action.into_mod_action()
    }

    #[tracing::instrument(skip_all)]
    pub async fn count_mod_actions(&self, user: UserId, action_type: ModActionType) -> Result<i32> {
        let id: i64 = user.into();
        let action_type = action_type.as_i32();
        Ok(sqlx::query_scalar!(
            "SELECT COUNT(*) FROM mod_action WHERE usr=? AND action_type=?",
            id,
            action_type
        )
        .fetch_one(&self.pool)
        .await?)
    }

    #[tracing::instrument(skip_all)]
    pub async fn count_all_mod_actions(&self, user: UserId) -> Result<HashMap<ModActionType, i32>> {
        let id: i64 = user.into();
        sqlx::query!(
            r#"SELECT action_type, COUNT(*) as "count!: i32" FROM mod_action WHERE usr=? GROUP BY action_type"#,
            id,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|x| Ok((ModActionType::from_i32(x.action_type as i32)?, x.count)))
        .collect::<Result<_>>()
    }

    #[tracing::instrument(skip_all, fields(mod_action.id = %id))]
    pub async fn remove_mod_action(&self, user: UserId, id: i64) -> Result<bool> {
        let user: i64 = user.into();
        let result = sqlx::query!("delete from mod_action where id=? AND usr=?", id, user)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    #[tracing::instrument(skip_all, fields(mod_action.id = %id, mod_action.moderator = %moderator.get(), mod_action.new_reason = %new_reason))]
    pub async fn edit_mod_action_reason(
        &self,
        id: i64,
        moderator: UserId,
        new_reason: String,
    ) -> Result<bool> {
        let moderator: i64 = moderator.into();
        let result = sqlx::query!(
            "update mod_action set reason=?, moderator=? where id=?",
            new_reason,
            moderator,
            id,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
