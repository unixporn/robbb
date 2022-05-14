use anyhow::Context;
use chrono::Utc;
use serenity::client;

use crate::modlog;

use super::*;

const TIMEOUT_MAX_DAYS: i64 = 28;

/// Mute a user for a given amount of time.
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::Helper }"
)]
pub async fn mute(
    ctx: Ctx<'_>,
    #[description = "User"] user: Member,
    #[description = "Duration of the mute"] duration: humantime::Duration,
    #[description = "Reason"]
    #[rest]
    reason: Option<String>,
) -> Res<()> {
    let success_msg = ctx
        .say_success_mod_action(format!("Muting {} for {}", user.mention(), duration))
        .await?
        .message()
        .await?;

    do_mute(
        ctx.discord(),
        ctx.guild().unwrap(),
        ctx.author().id,
        user.clone(),
        *duration,
        reason.clone(),
        Some(success_msg.link()),
    )
    .await?;

    modlog::log_mute(&ctx, &success_msg, &user.user, duration, reason).await;

    Ok(())
}

/// mute the user and add the mute-entry to the database.
pub async fn do_mute(
    ctx: &client::Context,
    guild: Guild,
    moderator: UserId,
    mut member: Member,
    duration: std::time::Duration,
    reason: Option<String>,
    context: Option<String>,
) -> anyhow::Result<()> {
    let db = ctx.get_db().await;

    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::from_std(duration).unwrap();

    // Ensure only one active mute per member
    db.remove_active_mutes(member.user.id).await?;

    db.add_mute(
        guild.id,
        moderator,
        member.user.id,
        reason.unwrap_or("no reason".to_string()),
        start_time,
        end_time,
        context,
    )
    .await?;

    // TODORW possibly make this actually work for longer timeouts, via re-adding the timeout
    // Also set a discord timeout when possible
    let latest_possible_timeout = Utc::now()
        .checked_add_signed(chrono::Duration::days(TIMEOUT_MAX_DAYS))
        .context("Overflow calculating max date")?
        .date();

    if end_time.date() <= latest_possible_timeout {
        member
            .disable_communication_until_datetime(&ctx, end_time.into())
            .await?;
    }

    set_mute_role(&ctx, member).await?;
    Ok(())
}

/// Adds the mute role to the user, but does _not_ add any database entry.
/// This should only be used if we know that an active database entry for the mute already exists,
/// or else we run the risk of accidentally muting someone forever.
pub async fn set_mute_role(ctx: &client::Context, mut member: Member) -> anyhow::Result<()> {
    let config = ctx.get_config().await;
    member.add_role(&ctx, config.role_mute).await?;
    Ok(())
}
