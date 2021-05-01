use super::*;

/// Restart the bot.
#[command]
#[only_in(guilds)]
#[usage("restart")]
pub async fn restart(ctx: &client::Context, msg: &Message) -> CommandResult {
    let _ = msg.reply(&ctx, "Shutting down").await;
    ctx.shard.shutdown_clean();

    std::process::exit(1);
}

/// Make the bot say something. Please don't actually use this :/
#[command]
#[only_in(guilds)]
#[usage("say <something>")]
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
#[only_in(guilds)]
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
#[only_in(guilds)]
#[usage("repo")]
pub async fn repo(ctx: &client::Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx, "https://github.com/unixporn/trup-rs")
        .await?;
    Ok(())
}

/// set your profiles description.
#[command]
#[only_in(guilds)]
#[usage("desc <text>")]
#[aliases("description")]
pub async fn desc(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    if let Some(value) = args.remains().map(|x| x.to_string()) {
        if value == "clear" {
            db.set_description(msg.author.id, None).await?;
            msg.reply_success(&ctx, "Successfully cleared your description!")
                .await?;
        } else {
            db.set_description(msg.author.id, Some(value)).await?;
            msg.reply_success(&ctx, "Successfully updated your description!")
                .await?;
        }
    } else if let Some(value) = db
        .get_profile(msg.author.id)
        .await?
        .and_then(|x| x.description)
    {
        msg.reply_embed(&ctx, |e| {
            e.description(value);
        })
        .await?;
    } else {
        msg.reply_error(&ctx, "You need to set your description first")
            .await?;
    }

    Ok(())
}

/// Provide a link to your git.
#[command]
#[only_in(guilds)]
#[usage("git <url>")]
pub async fn git(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    if let Some(value) = args.remains().map(|x| x.to_string()) {
        if value == "clear" {
            db.set_git(msg.author.id, None).await?;
            msg.reply_success(&ctx, "Successfully cleared your git-url!")
                .await?;
        } else {
            db.set_git(msg.author.id, Some(value)).await?;
            msg.reply_success(&ctx, "Successfully updated your git-url!")
                .await?;
        }
    } else if let Some(value) = db.get_profile(msg.author.id).await?.and_then(|x| x.git) {
        msg.reply_embed(&ctx, |e| {
            e.description(value);
        })
        .await?;
    } else {
        msg.reply_error(&ctx, "You need to set your git-link first")
            .await?;
    }
    Ok(())
}

/// Provide a link to your dotfiles
#[command]
#[only_in(guilds)]
#[usage("dotfiles <url>")]
pub async fn dotfiles(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    if let Some(value) = args.remains().map(|x| x.to_string()) {
        if value == "clear" {
            db.set_dotfiles(msg.author.id, None).await?;
            msg.reply_success(&ctx, "Successfully cleared your dotfiles!")
                .await?;
        } else {
            db.set_dotfiles(msg.author.id, Some(value)).await?;
            msg.reply_success(&ctx, "Successfully updated your dotfiles!")
                .await?;
        }
    } else if let Some(value) = db
        .get_profile(msg.author.id)
        .await?
        .and_then(|x| x.dotfiles)
    {
        msg.reply_embed(&ctx, |e| {
            e.description(value);
        })
        .await?;
    } else {
        msg.reply_error(&ctx, "You need to set your dotfiles first")
            .await?;
    }

    Ok(())
}

/// Sends the server invite link
#[command]
#[only_in(guilds)]
#[usage("invite")]
pub async fn invite(ctx: &client::Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx, "https://discord.gg/BVJEuY6yRc").await?;
    Ok(())
}
