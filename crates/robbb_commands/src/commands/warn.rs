use chrono::Utc;
use poise::serenity_prelude::User;

use crate::modlog;

use super::*;

/// Warn a user
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn warn(
    ctx: Ctx<'_>,
    #[description = "Who is the criminal?"]
    #[rename = "criminal"]
    user: User,
    #[description = "What did they do?"]
    #[rest]
    reason: String,
) -> Res<()> {
    let db = ctx.get_db();
    let warn_count = db.count_warns(user.id).await?;

    let success_msg = ctx
        .say_success_mod_action(format!(
            "{} has been warned by {} for the {} time for reason: {}",
            user.mention(),
            ctx.author().id.mention(),
            util::format_count(warn_count + 1),
            reason,
        ))
        .await?
        .message()
        .await?;

    db.add_warn(
        ctx.author().id,
        user.id,
        reason.to_string(),
        Utc::now(),
        Some(success_msg.link()),
    )
    .await?;

    modlog::log_warn(&ctx, &success_msg, user, warn_count, &reason).await;
    Ok(())
}

/// Undo the most recent warning on a user
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn undo_warn(
    ctx: Ctx<'_>,
    #[description = "Who was wrongfully convicted?"] user: User,
) -> Res<()> {
    let db = ctx.get_db();
    db.undo_latest_warn(user.id).await?;
    ctx.say_success_mod_action("Successfully removed the warning!")
        .await?;

    Ok(())
}
