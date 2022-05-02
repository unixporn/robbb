use super::*;

/// Mute a user for a given amount of time.
#[command]
#[only_in(guilds)]
#[usage("mute <user> <duration> [reason]")]
pub async fn mute(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = {
        let user_mention = args
            .single_quoted::<String>()
            .invalid_usage(&MUTE_COMMAND_OPTIONS)?;
        disambiguate_user_mention(&ctx, &guild, msg, &user_mention)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?
    };

    let mentioned_user = mentioned_user_id.to_user(&ctx).await?;

    let duration = args
        .single::<humantime::Duration>()
        .map_err(|_| UserErr::Other("Malformed duration".to_string()))?;

    let reason = args.remains();

    let guild = msg.guild(&ctx).await.context("Failed to fetch guild")?;
    let member = guild.member(&ctx, mentioned_user_id).await?;

    do_mute(
        &ctx,
        guild,
        msg.author.id,
        member,
        *duration,
        reason,
        Some(msg.link()),
    )
    .await?;

    msg.reply_success_mod_action(
        &ctx,
        format!(
            "{} has been muted for {}",
            mentioned_user_id.mention(),
            duration
        ),
    )
    .await?;

    modlog::log_mute(ctx, msg, &mentioned_user, duration, reason).await;

    Ok(())
}

/// mute the user and add the mute-entry to the database.
pub async fn do_mute(
    ctx: &client::Context,
    guild: Guild,
    moderator: UserId,
    member: Member,
    duration: std::time::Duration,
    reason: Option<&str>,
    context: Option<String>,
) -> Result<()> {
    let db = ctx.get_db().await;

    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::from_std(duration).unwrap();

    // Ensure only one active mute per member
    db.remove_active_mutes(member.user.id).await?;

    db.add_mute(
        guild.id,
        moderator,
        member.user.id,
        reason.unwrap_or("no reason").to_string(),
        start_time,
        end_time,
        context,
    )
    .await?;

    set_mute_role(&ctx, member).await?;
    Ok(())
}

/// Adds the mute role to the user, but does _not_ add any database entry.
/// This should only be used if we know that an active database entry for the mute already exists,
/// or else we run the risk of accidentally muting someone forever.
pub async fn set_mute_role(ctx: &client::Context, mut member: Member) -> Result<()> {
    let config = ctx.get_config().await;
    member.add_role(&ctx, config.role_mute).await?;
    Ok(())
}
