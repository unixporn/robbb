use super::*;

#[command]
#[usage("mute <user> <duration> [reason]")]
pub async fn mute(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let config = data.get::<Config>().unwrap().clone();
    let db = data.get::<Db>().unwrap().clone();

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = match args.single::<String>() {
        Ok(mentioned_user) => disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?,
        Err(_) => msg.author.id,
    };

    let duration = args
        .single::<humantime::Duration>()
        .map_err(|_| UserErr::Other("Malformed duration".to_string()))?;

    let reason = args.remains().unwrap_or_default();

    let guild = msg.guild(&ctx).await.context("Failed to fetch guild")?;
    let mut member = guild.member(&ctx, mentioned_user_id).await?;

    let start_time = Utc::now();
    let end_time = start_time + chrono::Duration::from_std(duration.into()).unwrap();

    db.add_mute(
        guild.id,
        msg.author.id,
        mentioned_user_id,
        reason.to_string(),
        start_time,
        end_time,
    )
    .await?;

    member.add_role(&ctx, config.role_mute).await?;

    msg.reply(
        &ctx,
        format!("{} has been muted for {}", member.mention(), duration),
    )
    .await?;

    config
        .log_bot_action(&ctx, |e| {
            e.description(format!(
                "User {} was muted by {}",
                mentioned_user_id.mention(),
                msg.author.id.mention(),
            ));
            e.field("Duration", duration, false);
            e.field("Reason", reason.to_string(), false);
        })
        .await;

    Ok(())
}
