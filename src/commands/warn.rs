use super::*;

/// Warn a user for a given reason.
#[command]
#[only_in(guilds)]
#[usage("warn <user> <reason> | warn undo <user>")]
#[sub_commands(undo_warn)]
pub async fn warn(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let (config, db) = ctx.get_config_and_db().await;

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let mentioned_user = &args
        .single_quoted::<String>()
        .invalid_usage(&WARN_COMMAND_OPTIONS)?;
    let mentioned_user_id = disambiguate_user_mention(&ctx, &guild, msg, mentioned_user)
        .await?
        .ok_or(UserErr::MentionedUserNotFound)?;

    let reason = args.remains().invalid_usage(&WARN_COMMAND_OPTIONS)?;

    db.add_warn(
        msg.author.id,
        mentioned_user_id,
        reason.to_string(),
        Utc::now(),
    )
    .await?;

    let warn_count = db.count_warns(mentioned_user_id).await?;

    let police_emote = ctx
        .get_up_emotes()
        .await
        .map(|x| format!(" {}", x.police))
        .unwrap_or_default();

    let _ = msg
        .reply(
            &ctx,
            format!(
                "{} has been warned by {} for the {} time for reason: {}{}",
                mentioned_user_id.mention(),
                msg.author.id.mention(),
                util::format_count(warn_count),
                reason,
                &police_emote,
            ),
        )
        .await;

    config
        .log_bot_action(&ctx, |e| {
            e.description(format!(
                "{} was warned by {} _({} warn)_\n{}",
                mentioned_user_id.mention(),
                msg.author.id.mention(),
                util::format_count(warn_count),
                msg.to_context_link(),
            ));
            e.field("Reason", reason, false);
        })
        .await;
    Ok(())
}

/// Undo the most recent warning on a user
#[command("undo")]
#[usage("warn undo <user>")]
pub async fn undo_warn(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let mentioned_user = &args
        .single_quoted::<String>()
        .invalid_usage(&WARN_COMMAND_OPTIONS)?;
    let mentioned_user_id = disambiguate_user_mention(&ctx, &guild, msg, mentioned_user)
        .await?
        .ok_or(UserErr::MentionedUserNotFound)?;

    let db = ctx.get_db().await;
    db.undo_latest_warn(mentioned_user_id).await?;

    msg.reply_success(&ctx, "Successfully removed the warning!")
        .await?;

    Ok(())
}
