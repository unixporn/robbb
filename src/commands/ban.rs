use super::*;
/// Ban a user from the server
#[command]
#[usage("ban <user> <reason>")]
pub async fn ban(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    do_ban(ctx, msg, args, 1).await?;
    Ok(())
}

/// Ban a user from the server, deleting all messages the user sent within the last day.
#[command]
#[usage("delban <user> <reason>")]
pub async fn delban(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    do_ban(ctx, msg, args, 1).await?;
    Ok(())
}

async fn do_ban(
    ctx: &client::Context,
    msg: &Message,
    mut args: Args,
    delete_days: u8,
) -> CommandResult {
    let data = ctx.data.read().await;
    let config = data.get::<Config>().unwrap().clone();

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let mentioned_user = &args
        .single::<UserId>()
        .invalid_usage(&BAN_COMMAND_OPTIONS)?;

    let reason = match args.remains() {
        Some(reason) => reason,
        None => error_out!(UserErr::invalid_usage(&BAN_COMMAND_OPTIONS)),
    };

    let user = mentioned_user
        .to_user(&ctx)
        .await
        .context("Failed to retrieve user for banned user")?;

    let _ = user
        .dm(&ctx, |m| {
            m.embed(|e| {
                e.title(format!("You where banned from {}", guild.name));
                e.field("Reason", reason, false)
            })
        })
        .await;

    guild
        .ban_with_reason(&ctx, mentioned_user, delete_days, reason)
        .await?;

    config
        .log_bot_action(&ctx, |e| {
            e.title("User banned").description(format!(
                "{} ({}) was banned by {}",
                user.mention(),
                user.name_with_disc(),
                msg.author.mention()
            ));
            e.field("Reason", reason, false);
        })
        .await;

    msg.reply_embed(&ctx, |e| {
        e.title("Successfully yeeted!");
    })
    .await?;
    Ok(())
}
