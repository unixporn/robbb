use anyhow::Context;
use serenity::all::User;

use crate::modlog;

use super::*;

/// Unban a user.
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn unban(
    ctx: Ctx<'_>,
    #[description = "ID of the user you want to unban"] user: User,
) -> Res<()> {
    let guild = ctx.guild().context("Failed to load guild")?.to_owned();
    guild.unban(&ctx.serenity_context(), user.id).await.with_user_error(|e| e.to_string())?;

    ctx.say_success(format!("Succesfully deyote {}", user.id.mention())).await?;

    modlog::log_unban(ctx, user).await;

    Ok(())
}
