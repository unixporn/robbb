use anyhow::Context;

use crate::modlog;

use super::*;

/// Unban a user.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator"
)]
pub async fn unban(
    ctx: Ctx<'_>,
    #[description = "ID of the user you want to unban"]
    #[rename = "id"]
    user_id: UserId,
) -> Res<()> {
    let guild = ctx.guild().context("Failed to load guild")?;
    let user = user_id.to_user(&ctx.discord()).await?;

    guild.unban(&ctx.discord(), user_id).await?;

    ctx.say_success(format!("Succesfully deyote {}", user_id.mention()))
        .await?;

    modlog::log_unban(ctx, user).await;

    Ok(())
}
