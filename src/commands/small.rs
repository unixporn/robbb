use super::*;

/// Restart the bot.
#[command]
#[usage("restart")]
pub async fn restart(ctx: &client::Context, msg: &Message) -> CommandResult {
    let _ = msg.reply(&ctx, "Shutting down").await;
    ctx.shard.shutdown_clean();

    std::process::exit(1);
}

/// Make the bot say something.
#[command]
#[usage("say <text>")]
pub async fn say(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let content = args.remains().invalid_usage(&SAY_COMMAND_OPTIONS)?;
    msg.channel_id
        .send_message(&ctx, |m| m.content(content))
        .await?;
    msg.delete(&ctx).await?;
    Ok(())
}

/// Print bot's latency to discord.
#[command]
#[usage("latency")]
pub async fn latency(ctx: &client::Context, msg: &Message) -> CommandResult {
    let msg_time = msg.timestamp;
    let now = Utc::now();
    let latency = now.timestamp_millis() - msg_time.timestamp_millis();
    msg.reply(&ctx, format!("Latency is **{}ms**", latency))
        .await?;

    Ok(())
}

/// Sends a link to the bot's repository! Feel free contribute!
#[command]
#[usage("repo")]
pub async fn repo(ctx: &client::Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx, "https://github.com/unixporn/trup-rs")
        .await?;
    Ok(())
}
