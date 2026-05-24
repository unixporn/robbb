use robbb_util::{
    extensions::PoiseContextExt as _,
    prelude::{Ctx, Res},
};
use serenity::all::MessageId;

use crate::{
    checks::{self, PermissionLevel},
    commands::OptionExt as _,
};

/// Pin or Unpin a message. In threads, this may also be ran by the thread owner!
#[poise::command(slash_command, guild_only, subcommands("pin_add", "pin_remove"))]
pub async fn pin_message(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Pin a message in this channel.
#[poise::command(slash_command, guild_only, rename = "add")]
pub async fn pin_add(
    ctx: Ctx<'_>,
    #[description = "The ID of the message that should be pinned"] message: MessageId,
) -> Res<()> {
    let channel = ctx.guild_channel().await.user_error("not inside a guild channel")?;
    let message = channel
        .message(&ctx, message)
        .await
        .user_error("Message ID does not exist or is not in this channel")?;

    let author_permissions =
        checks::get_permission_level(ctx.serenity_context(), ctx.author()).await?;

    if author_permissions != PermissionLevel::Mod && (Some(ctx.author().id) != channel.owner_id) {
        ctx.say_error("You are not allowed to do that :<").await?;
        return Ok(());
    }

    message.pin(&ctx).await.user_error("Failed to pin the message")?;
    ctx.reply_embed_ephemeral_builder(|e| e.description("Message has been pinned!")).await?;

    Ok(())
}

/// Unpin a message in this channel.
#[poise::command(slash_command, guild_only, rename = "remove")]
pub async fn pin_remove(
    ctx: Ctx<'_>,
    #[description = "The ID of the message that should be unpinned"] message: MessageId,
) -> Res<()> {
    let channel = ctx.guild_channel().await.user_error("not inside a guild channel")?;
    let message = channel
        .message(&ctx, message)
        .await
        .user_error("Message ID does not exist or is not in this channel")?;

    let author_permissions =
        checks::get_permission_level(ctx.serenity_context(), ctx.author()).await?;

    if author_permissions != PermissionLevel::Mod && (Some(ctx.author().id) != channel.owner_id) {
        ctx.say_error("You are not allowed to do that :<").await?;
        return Ok(());
    }

    message.unpin(&ctx).await.user_error("Failed to unpin the message")?;
    ctx.reply_embed_ephemeral_builder(|e| e.description("Message has been unpinned!")).await?;

    Ok(())
}
