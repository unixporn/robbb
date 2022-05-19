use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

use super::Db;

#[derive(Debug, Eq, Copy, Clone, PartialEq, Hash)]
pub enum NoteType {
    ManualNote,
    BlocklistViolation,
    Warn,
    Mute,
    Ban,
    Kick,
}
impl NoteType {
    fn from_i32(n: i32) -> Result<Self> {
        match n {
            0 => Ok(NoteType::ManualNote),
            1 => Ok(NoteType::BlocklistViolation),
            2 => Ok(NoteType::Warn),
            3 => Ok(NoteType::Mute),
            4 => Ok(NoteType::Ban),
            5 => Ok(NoteType::Kick),
            _ => Err(anyhow!("Invalid note type: {}", n)),
        }
    }

    fn as_i32(&self) -> i32 {
        match self {
            NoteType::ManualNote => 0,
            NoteType::BlocklistViolation => 1,
            NoteType::Warn => 2,
            NoteType::Mute => 3,
            NoteType::Ban => 4,
            NoteType::Kick => 5,
        }
    }
}

impl std::fmt::Display for NoteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoteType::ManualNote => write!(f, "Moderator Note"),
            NoteType::BlocklistViolation => write!(f, "[AUTO] - Blocklist Violation"),
            NoteType::Warn => write!(f, "Warning"),
            NoteType::Mute => write!(f, "Mute"),
            NoteType::Ban => write!(f, "Ban"),
            NoteType::Kick => write!(f, "Kick"),
        }
    }
}

#[derive(Debug)]
pub struct Note {
    pub id: i64,
    pub moderator: UserId,
    pub user: UserId,
    pub content: String,
    pub create_date: DateTime<Utc>,
    pub note_type: NoteType,
    pub context: Option<String>,
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn add_note(
        &self,
        moderator: UserId,
        user: UserId,
        content: String,
        create_date: DateTime<Utc>,
        note_type: NoteType,
        context: Option<String>,
    ) -> Result<Note> {
        let mut conn = self.pool.acquire().await?;
        let id = {
            let moderator = moderator.0 as i64;
            let user = user.0 as i64;
            let note_type = note_type.as_i32();
            sqlx::query!(
                "insert into note (moderator, usr, content, create_date, note_type, context) values(?, ?, ?, ?, ?, ?)",
                moderator,
                user,
                content,
                create_date,
                note_type,
                context,
            )
            .execute(&mut conn)
            .await?
            .last_insert_rowid()
        };

        Ok(Note {
            id,
            moderator,
            user,
            content,
            create_date,
            note_type,
            context,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn undo_latest_note(&self, user: UserId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let user = user.0 as i64;
        sqlx::query!(
            r#"delete from note as n where usr=? and create_date=(select max(create_date) from note where usr=n.usr)"#,
            user,
        ).execute(&mut conn).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_notes(&self, user_id: UserId, filter: Option<NoteType>) -> Result<Vec<Note>> {
        let mut conn = self.pool.acquire().await?;
        let user_id = user_id.0 as i64;

        let note_type_value = filter.map(|x| x.as_i32());
        sqlx::query!(
            r#"
                SELECT * FROM note
                WHERE usr=?1 AND (?2 IS NULL OR note_type=?2)
                ORDER BY create_date DESC"#,
            user_id,
            note_type_value,
        )
        .fetch_all(&mut conn)
        .await?
        .into_iter()
        .map(|x| {
            Ok(Note {
                id: x.id,
                moderator: UserId(x.moderator as u64),
                user: UserId(x.usr as u64),
                content: x.content,
                create_date: chrono::DateTime::from_utc(x.create_date, Utc),
                note_type: NoteType::from_i32(x.note_type as i32)?,
                context: x.context,
            })
        })
        .collect::<Result<_>>()
    }
}
