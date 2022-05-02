use chrono::Utc;
use poise::{
    serenity_prelude::{Mentionable, User, UserId},
    Modal,
};

use crate::{
    abort_with,
    db::{note::NoteType, Db},
    embeds, modlog,
};

use super::*;

// TODORW probably use subcommands here

#[derive(Debug, Modal)]
#[name = "Take a note"]
struct NoteModal {
    #[name = "Note"]
    #[paragraph]
    note: String,
}

/// Write a note about a user.
#[poise::command(slash_command, guild_only, prefix_command, track_edits)]
pub async fn note(
    ctx: Ctx<'_>,
    #[description = "User"] user: User,
    #[description = "The note"]
    #[rest]
    content: Option<String>,
) -> Res<()> {
    let db = ctx.get_db();

    let content = match (content, ctx) {
        (Some(content), _) => content,
        (None, poise::Context::Application(ctx)) => NoteModal::execute(ctx).await?.note,
        (None, poise::Context::Prefix(_)) => {
            abort_with!(UserErr::InvalidUsage("No note content provided"))
        }
    };

    let success_msg = ctx.say_success("Noting...").await?.message().await.ok();

    db.add_note(
        ctx.author().id,
        user.id,
        content.to_string(),
        Utc::now(),
        NoteType::ManualNote,
        success_msg.map(|x| x.link()),
    )
    .await?;

    modlog::log_note(ctx, &user, &content).await;
    Ok(())
}

/// Undo the most recent note of a user
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    track_edits,
    rename = "undo-note"
)]
pub async fn undo_note(ctx: Ctx<'_>, #[description = "User"] user: User) -> Res<()> {
    let db = ctx.get_db();
    db.undo_latest_note(user.id).await?;
    ctx.say_success("Successfully removed the note!").await?;

    Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum NoteFilterParam {
    Mod,
    Blocklist,
    Warn,
    Mute,
    Ban,
    Kick,
    All,
}

/// Read notes about a user.
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    track_edits,
    rename = "notes"
)]
pub async fn notes(
    ctx: Ctx<'_>,
    #[description = "User"] user: User,
    #[description = "What kind of notes to show"] note_filter: Option<NoteFilterParam>,
) -> Res<()> {
    let db = ctx.get_db();

    let note_filter = match note_filter {
        Some(NoteFilterParam::Mod) => Some(NoteType::ManualNote),
        Some(NoteFilterParam::Blocklist) => Some(NoteType::BlocklistViolation),
        Some(NoteFilterParam::Warn) => Some(NoteType::Warn),
        Some(NoteFilterParam::Mute) => Some(NoteType::Mute),
        Some(NoteFilterParam::Ban) => Some(NoteType::Ban),
        Some(NoteFilterParam::Kick) => Some(NoteType::Kick),
        _ => None,
    };

    let avatar_url = user.face();

    let notes = fetch_note_values(&db, user.id, note_filter).await?;

    let fields = notes.iter().map(|note| {
        let context_link = note
            .context
            .clone()
            .map(|link| format!(" - [(context)]({})", link))
            .unwrap_or_else(String::new);
        (
            format!("{} - {} ", note.note_type, util::format_date_ago(note.date)),
            format!(
                "{} - {}{}",
                note.description,
                note.moderator.mention(),
                context_link
            ),
        )
    });

    let base_embed = embeds::make_create_embed(ctx.discord(), |e| {
        e.title("Notes")
            .description(format!("Notes about {}", user.mention()))
            .author(|a| a.icon_url(avatar_url))
    })
    .await;

    embeds::PaginatedEmbed::create_from_fields(fields, base_embed)
        .await
        .reply_to(ctx)
        .await?;

    Ok(())
}

#[allow(dead_code)]
struct NotesEntry {
    note_type: NoteType,
    description: String,
    date: chrono::DateTime<Utc>,
    moderator: UserId,
    context: Option<String>,
}

#[allow(unused)]
async fn fetch_note_values(
    db: &Db,
    user_id: UserId,
    filter: Option<NoteType>,
) -> Res<Vec<NotesEntry>> {
    let mut entries = Vec::new();

    if filter.is_none()
        || filter == Some(NoteType::ManualNote)
        || filter == Some(NoteType::BlocklistViolation)
    {
        let notes = db
            .get_notes(user_id, filter)
            .await?
            .into_iter()
            .map(|x| NotesEntry {
                note_type: x.note_type,
                description: x.content,
                moderator: x.moderator,
                date: x.create_date,
                context: x.context,
            });
        entries.extend(notes);
    }
    if filter.is_none() || filter == Some(NoteType::Mute) {
        let mutes = db
            .get_mutes(user_id)
            .await?
            .into_iter()
            .map(|x| NotesEntry {
                note_type: NoteType::Mute,
                description: format!(
                    "[{}] {}",
                    humantime::Duration::from(
                        (x.end_time - x.start_time).to_std().unwrap_or_default()
                    ),
                    x.reason
                ),
                date: x.start_time,
                moderator: x.moderator,
                context: x.context,
            });
        entries.extend(mutes);
    }
    if filter.is_none() || filter == Some(NoteType::Warn) {
        let warns = db
            .get_warns(user_id)
            .await?
            .into_iter()
            .map(|x| NotesEntry {
                note_type: NoteType::Warn,
                description: x.reason,
                date: x.create_date,
                moderator: x.moderator,
                context: x.context,
            });
        entries.extend(warns);
    }
    if filter.is_none() || filter == Some(NoteType::Ban) {
        let bans = db.get_bans(user_id).await?.into_iter().map(|x| NotesEntry {
            note_type: NoteType::Ban,
            description: x.reason,
            date: x.create_date,
            moderator: x.moderator,
            context: x.context,
        });
        entries.extend(bans);
    }
    if filter.is_none() || filter == Some(NoteType::Kick) {
        let kicks = db
            .get_kicks(user_id)
            .await?
            .into_iter()
            .map(|x| NotesEntry {
                note_type: NoteType::Kick,
                description: x.reason,
                date: x.create_date,
                moderator: x.moderator,
                context: x.context,
            });
        entries.extend(kicks);
    }
    entries.sort_by_key(|x| x.date);
    Ok(entries)
}
