use chrono::Utc;
use eyre::ContextCompat as _;
use poise::serenity_prelude::User;
use serenity::{all::GuildId, builder::CreateEmbed, client};

use crate::modlog;

use super::*;

/// Kick a user from the server
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
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
    let guild = ctx.guild().context("Failed to fetch guild")?.clone();
    do_kick(ctx.serenity_context(), guild.id, &user, &reason).await?;

    let success_msg = ctx
        .say_success_mod_action(format!("{} has been kicked from the server", user.id.mention()))
        .await?;
    let success_msg = success_msg.message().await?;

    db.add_mod_action(
        ctx.author().id,
        user.id,
        reason.to_string(),
        Utc::now(),
        success_msg.link(),
        robbb_db::mod_action::ModActionKind::Kick,
    )
    .await?;

    modlog::log_kick(ctx, &success_msg, user, &reason).await;

    Ok(())
}

pub async fn do_kick(ctx: &client::Context, guild: GuildId, user: &User, reason: &str) -> Res<()> {
    let _ = user
        .dm(
            &ctx,
            CreateEmbed::default()
                .title("You were kicked")
                .field("Reason", reason, false)
                .into_create_message(),
        )
        .await;
    guild.kick_with_reason(&ctx, user, reason).await?;
    Ok(())
}
