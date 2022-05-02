use super::*;

/// Kick a user from the server
#[command]
#[only_in(guilds)]
#[usage("kick <user> <reason>")]
pub async fn kick(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let (_config, db) = ctx.get_config_and_db().await;

    let guild = msg.guild(&ctx).context("Failed to load guild")?;

    let mentioned_user_id = {
        let user_mention = args
            .single_quoted::<String>()
            .invalid_usage(&KICK_COMMAND_OPTIONS)?;
        disambiguate_user_mention(&ctx, &guild, msg, &user_mention)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?
    };

    let mentioned_user = mentioned_user_id.to_user(&ctx).await?;

    let reason = args.remains().unwrap_or("no reason");

    do_kick(&ctx, guild, &mentioned_user_id, reason).await?;

    db.add_kick(
        msg.author.id,
        mentioned_user_id,
        reason.to_string(),
        Utc::now(),
        Some(msg.link()),
    )
    .await?;

    msg.reply_success_mod_action(
        &ctx,
        format!(
            "{} has been kicked from the server",
            mentioned_user_id.mention()
        ),
    )
    .await?;

    modlog::log_kick(&ctx, msg, mentioned_user, reason).await;

    Ok(())
}

pub async fn do_kick(
    ctx: &client::Context,
    guild: Guild,
    mentioned_user: &UserId,
    reason: &str,
) -> Result<()> {
    let user = mentioned_user
        .to_user(&ctx)
        .await
        .context("Failed to retrieve user for kicked user")?;
    let _ = user
        .dm(&ctx, |m| -> &mut serenity::builder::CreateMessage {
            m.embed(|e| {
                e.title(format!("You were kicked from {}", guild.name));
                e.field("Reason", reason, false)
            })
        })
        .await;
    guild.kick_with_reason(&ctx, mentioned_user, reason).await?;
    Ok(())
}
