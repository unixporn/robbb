use chrono::Utc;
use poise::{
    serenity_prelude::{Mentionable, User},
    Modal,
};
use robbb_db::mod_action::ModActionType;
use robbb_util::embeds;

use crate::modlog;

use super::*;

/// Write a note about a user.
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    subcommands("note_add", "note_list", "note_delete", "note_edit")
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

    let success_msg = ctx.say_success("Noting...").await?;
    let success_msg = success_msg.message().await?;

    db.add_mod_action(
        ctx.author().id,
        user.id,
        content.to_string(),
        Utc::now(),
        success_msg.link(),
        robbb_db::mod_action::ModActionKind::ManualNote,
    )
    .await?;

    modlog::log_note(ctx, &user, &content).await;
    Ok(())
}

/// Remove a specific mod action
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    rename = "delete"
)]
pub async fn note_delete(
    ctx: Ctx<'_>,
    #[description = "The user"] user: User,
    #[description = "Id of the mod action"] id: i64,
) -> Res<()> {
    let db = ctx.get_db();
    let succeeded = db.remove_mod_action(user.id, id).await?;
    if succeeded {
        ctx.say_success_mod_action("Successfully removed the entry!").await?;
    } else {
        ctx.say_error("No action with that id and user").await?;
    }
    Ok(())
}

/// Edit a mod action
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    rename = "edit"
)]
pub async fn note_edit(
    app_ctx: AppCtx<'_>,
    #[description = "Id of the mod action"] id: i64,
) -> Res<()> {
    let ctx = Ctx::Application(app_ctx);
    let db = ctx.get_db();
    let action = db.get_mod_action(id).await?;

    #[derive(Modal)]
    #[name = "Edit"]
    struct NoteEditModal {
        #[paragraph]
        reason: String,
    }

    let NoteEditModal { reason } =
        NoteEditModal::execute_with_defaults(app_ctx, NoteEditModal { reason: action.reason })
            .await?;
    let reason = reason.trim().trim_matches('\n');

    db.edit_mod_action_reason(action.id, ctx.author().id, reason.to_string()).await?;
    ctx.say_success_mod_action(format!(
        "Successfully edited {}'s entry {}",
        action.user.mention(),
        action.id
    ))
    .await?;
    Ok(())
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
    #[description = "What kind of notes to show"] note_filter: Option<ModActionType>,
) -> Res<()> {
    let db = ctx.get_db();

    let mut notes = db.get_mod_actions(user.id, note_filter).await?;
    notes.sort_by_key(|x| std::cmp::Reverse(x.create_date));

    let fields = notes.iter().map(|note| {
        let context_link = note
            .context
            .clone()
            .map(|link| format!(" - [(context)]({})", link))
            .unwrap_or_else(String::new);
        (
            format!(
                "[{}] {} - {} ",
                note.id,
                note.kind.to_action_type(),
                util::format_date_ago(note.create_date.unwrap_or_else(Utc::now))
            ),
            format!("{} - {}{}", note.reason, note.moderator.mention(), context_link),
        )
    });

    let base_embed = embeds::make_create_embed(ctx.discord(), |e| {
        e.description(format!("{} notes about {}", notes.len(), user.mention()));
        e.author_user(&user)
    })
    .await;

    embeds::PaginatedEmbed::create_from_fields("Notes".to_string(), fields, base_embed)
        .await
        .reply_to(ctx, false)
        .await?;

    Ok(())
}
