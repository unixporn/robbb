use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

pub struct ModAction {
    pub id: i64,
    pub moderator: UserId,
    pub user: UserId,
    pub reason: String,
    pub create_date: Option<DateTime<Utc>>,
    pub context: Option<String>,
    pub kind: ModActionKind,
}

pub enum ModActionKind {
    ManualNote,
    BlocklistViolation,
    Warn,
    Mute {
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        active: bool,
    },
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

#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash)]
pub enum ModActionType {
    ManualNote,
    BlocklistViolation,
    Warn,
    Mute,
    Ban,
    Kick,
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

impl std::fmt::Display for ModActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModActionType::ManualNote => write!(f, "Moderator Note"),
            ModActionType::BlocklistViolation => write!(f, "[AUTO] - Blocklist Violation"),
            ModActionType::Warn => write!(f, "Warning"),
            ModActionType::Mute => write!(f, "Mute"),
            ModActionType::Ban => write!(f, "Ban"),
            ModActionType::Kick => write!(f, "Kick"),
        }
    }
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn add_mod_action(
        &self,
        moderator: UserId,
        user: UserId,
        reason: String,
        create_date: DateTime<Utc>,
        context: Option<String>,
        kind: ModActionKind,
    ) -> Result<ModAction> {
        let mut trans = self.pool.begin().await?;

        let id = {
            let moderator = moderator.0 as i64;
            let user = user.0 as i64;
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
            .execute(&mut trans)
            .await?
            .last_insert_rowid()
        };

        if let ModActionKind::Mute {
            start_time,
            end_time,
            active,
        } = kind
        {
            sqlx::query!(
                "insert into mute (mod_action, start_time, end_time, active) VALUES(?, ?, ?, ?)",
                id,
                start_time,
                end_time,
                active
            )
            .execute(&mut trans)
            .await?;
        }
        trans.commit().await?;

        Ok(ModAction {
            id,
            moderator,
            user,
            reason,
            create_date: Some(create_date),
            context,
            kind,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_mod_actions(
        &self,
        user_id: UserId,
        filter: Option<ModActionType>,
    ) -> Result<Vec<ModAction>> {
        let mut conn = self.pool.acquire().await?;
        let user_id = user_id.0 as i64;

        let note_type_value = filter.map(|x| x.as_i32());

        sqlx::query!(
            r#"
                SELECT * FROM mod_action
                LEFT JOIN mute ON mod_action.id = mute.mod_action
                WHERE usr=?1 AND (?2 IS NULL OR action_type=?2)
                ORDER BY create_date DESC"#,
            user_id,
            note_type_value,
        )
        .fetch_all(&mut conn)
        .await?
        .into_iter()
        .map(|x| {
            Ok(ModAction {
                id: x.id,
                moderator: UserId(x.moderator as u64),
                user: UserId(x.usr as u64),
                reason: x.reason.unwrap_or_default(),
                create_date: x.create_date.map(|x| chrono::DateTime::from_utc(x, Utc)),
                context: x.context,
                kind: match ModActionType::from_i32(x.action_type as i32)? {
                    ModActionType::ManualNote => ModActionKind::ManualNote,
                    ModActionType::BlocklistViolation => ModActionKind::BlocklistViolation,
                    ModActionType::Warn => ModActionKind::Warn,
                    ModActionType::Mute => ModActionKind::Mute {
                        start_time: chrono::DateTime::from_utc(
                            x.start_time.context("no mute item for mute in database")?,
                            Utc,
                        ),
                        end_time: chrono::DateTime::from_utc(
                            x.end_time.context("no mute item for mute in database")?,
                            Utc,
                        ),
                        active: x.active,
                    },
                    ModActionType::Ban => ModActionKind::Ban,
                    ModActionType::Kick => ModActionKind::Kick,
                },
            })
        })
        .collect::<Result<_>>()
    }

    pub async fn count_mod_actions(&self, user: UserId, action_type: ModActionType) -> Result<i32> {
        let mut conn = self.pool.acquire().await?;
        let id = user.0 as i64;
        let action_type = action_type.as_i32();
        Ok(sqlx::query_scalar!(
            "select count(*) from mod_action where usr=? AND action_type=?",
            id,
            action_type
        )
        .fetch_one(&mut conn)
        .await?)
    }

    pub async fn undo_latest_mod_action(
        &self,
        user: UserId,
        action_type: ModActionType,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let user = user.0 as i64;
        let action_type = action_type.as_i32();
        sqlx::query!(
            r#"delete from mod_action as a
            where usr=? AND action_type=? AND create_date=(select max(create_date) from mod_action where usr=a.usr)"#,
            user,
            action_type
        ).execute(&mut conn).await?;
        Ok(())
    }
}
