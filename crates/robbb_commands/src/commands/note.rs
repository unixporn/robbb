use chrono::Utc;
use poise::serenity_prelude::{Mentionable, User, UserId};
use robbb_db::{
    mod_action::{ModAction, ModActionType},
    Db,
};
use robbb_util::embeds;

use crate::modlog;

use super::*;

/// Write a note about a user.
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    subcommands("note_add", "note_undo", "note_list")
)]
pub async fn note(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Write a note about a user.
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    rename = "add"
)]
pub async fn note_add(
    ctx: Ctx<'_>,
    #[description = "User"] user: User,
    #[rest]
    #[description = "The note"]
    content: String,
) -> Res<()> {
    let db = ctx.get_db();

    let success_msg = ctx.say_success("Noting...").await?.message().await.ok();

    db.add_mod_action(
        ctx.author().id,
        user.id,
        content.to_string(),
        Utc::now(),
        success_msg.map(|x| x.link()),
        robbb_db::mod_action::ModActionKind::ManualNote,
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
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    rename = "undo"
)]
pub async fn note_undo(ctx: Ctx<'_>, #[description = "User"] user: User) -> Res<()> {
    let db = ctx.get_db();
    db.undo_latest_mod_action(user.id, ModActionType::ManualNote)
        .await?;
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
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    rename = "list"
)]
pub async fn note_list(
    ctx: Ctx<'_>,
    #[description = "User"] user: User,
    #[description = "What kind of notes to show"] note_filter: Option<NoteFilterParam>,
) -> Res<()> {
    let db = ctx.get_db();

    let note_filter = match note_filter {
        Some(NoteFilterParam::Mod) => Some(ModActionType::ManualNote),
        Some(NoteFilterParam::Blocklist) => Some(ModActionType::BlocklistViolation),
        Some(NoteFilterParam::Warn) => Some(ModActionType::Warn),
        Some(NoteFilterParam::Mute) => Some(ModActionType::Mute),
        Some(NoteFilterParam::Ban) => Some(ModActionType::Ban),
        Some(NoteFilterParam::Kick) => Some(ModActionType::Kick),
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
            format!(
                "{} - {} ",
                note.kind.to_action_type(),
                util::format_date_ago(note.create_date.unwrap_or_else(|| Utc::now()))
            ),
            format!(
                "{} - {}{}",
                note.reason,
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
        .reply_to(ctx, false)
        .await?;

    Ok(())
}

pub struct NotesEntry {
    pub note_type: ModActionType,
    pub description: String,
    pub date: chrono::DateTime<Utc>,
    pub moderator: UserId,
    pub context: Option<String>,
}

pub async fn fetch_note_values(
    db: &Db,
    user_id: UserId,
    filter: Option<ModActionType>,
) -> Res<Vec<ModAction>> {
    let mut entries = db.get_mod_actions(user_id, filter).await?;
    entries.sort_by_key(|x| x.create_date);
    Ok(entries)
}
