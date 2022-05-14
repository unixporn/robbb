use anyhow::Context;
use chrono::Utc;
use poise::serenity_prelude::User;
use serenity::client;

use crate::modlog;

use super::*;

/// Kick a user from the server
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn kick(
    ctx: Ctx<'_>,
    #[description = "Who is the criminal?"]
    #[rename = "criminal"]
    user: User,
    #[description = "What did they do?"]
    #[rest]
    reason: String,
) -> Res<()> {
    let db = ctx.get_db();
    let guild = ctx.guild().context("Failed to fetch guild")?;
    do_kick(&ctx.discord(), guild, &user, &reason).await?;

    let success_msg = ctx
        .say_success_mod_action(format!(
            "{} has been kicked from the server",
            user.id.mention()
        ))
        .await?
        .message()
        .await?;

    db.add_kick(
        ctx.author().id,
        user.id,
        reason.to_string(),
        Utc::now(),
        Some(success_msg.link()),
    )
    .await?;

    modlog::log_kick(ctx, &success_msg, user, &reason).await;

    Ok(())
}

pub async fn do_kick(ctx: &client::Context, guild: Guild, user: &User, reason: &str) -> Res<()> {
    let _ = user
        .dm(&ctx, |m| -> &mut serenity::builder::CreateMessage {
            m.embed(|e| {
                e.title(format!("You were kicked from {}", guild.name));
                e.field("Reason", reason, false)
            })
        })
        .await;
    guild.kick_with_reason(&ctx, user, reason).await?;
    Ok(())
}
