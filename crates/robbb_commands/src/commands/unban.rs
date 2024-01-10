use anyhow::Context;

use crate::modlog;

use super::*;

/// Unban a user.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn unban(
    ctx: Ctx<'_>,
    #[description = "ID of the user you want to unban"]
    #[rename = "id"]
    user_id: UserId,
) -> Res<()> {
    let user = user_id.to_user(&ctx.serenity_context()).await?;
    let guild = ctx.guild().context("Failed to load guild")?.to_owned();
    guild.unban(&ctx.serenity_context(), user_id).await?;

    ctx.say_success(format!("Succesfully deyote {}", user_id.mention())).await?;

    modlog::log_unban(ctx, user).await;

    Ok(())
}
