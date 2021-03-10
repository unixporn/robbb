use super::*;

#[command]
#[usage("mute <user> <duration> [reason]")]
pub async fn mute(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let config = data.get::<Config>().unwrap().clone();
    let db = data.get::<Db>().unwrap().clone();

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = match args.single_quoted::<String>() {
        Ok(mentioned_user) => disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?,
        Err(_) => msg.author.id,
    };

    let duration = args
        .single::<humantime::Duration>()
        .map_err(|_| UserErr::Other("Malformed duration".to_string()))?;

    let reason = args.remains();

    let guild = msg.guild(&ctx).await.context("Failed to fetch guild")?;
    let mut member = guild.member(&ctx, mentioned_user_id).await?;

    do_mute(&ctx, guild, msg.author.id, member, *duration, reason).await?;

    msg.reply_success(
        &ctx,
        format!(
            "{} has been muted for {}",
            mentioned_user_id.mention(),
            duration
        ),
    )
    .await?;

    config
        .log_bot_action(&ctx, |e| {
            e.description(format!(
                "User {} was muted by {}\n{}",
                mentioned_user_id.mention(),
                msg.author.id.mention(),
                msg.to_context_link(),
            ));
            e.field("Duration", duration, false);
            reason.map(|r| e.field("Reason", r, false));
        })
        .await;

    Ok(())
}

pub async fn do_mute(
    ctx: &client::Context,
    guild: Guild,
    moderator: UserId,
    mut member: Member,
    duration: std::time::Duration,
    reason: Option<&str>,
) -> Result<()> {
    let data = ctx.data.read().await;
    let config = data.get::<Config>().unwrap().clone();
    let db = data.get::<Db>().unwrap().clone();

    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::from_std(duration.into()).unwrap();

    db.add_mute(
        guild.id,
        moderator,
        member.user.id,
        reason.unwrap_or("no reason").to_string(),
        start_time,
        end_time,
    )
    .await?;

    member.add_role(&ctx, config.role_mute).await?;
    Ok(())
}
