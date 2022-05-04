use chrono::Utc;

use super::*;

/// Restart robbb
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    check = "crate::checks::check_is_moderator"
)]
pub async fn restart(ctx: Ctx<'_>) -> Res<()> {
    let _ = ctx.say_success("Shutting down").await;
    ctx.discord().shard.shutdown_clean();
    std::process::exit(1);
}

/// Make the bot say something. Please don't actually use this :/
#[poise::command(slash_command, guild_only, check = "crate::checks::check_is_moderator")]
pub async fn say(
    ctx: Ctx<'_>,
    #[description = "What you,.. ummmm. I mean _I_ should say"] message: String,
) -> Res<()> {
    ctx.channel_id().say(&ctx.discord(), message).await?;
    Ok(())
}

// TODORW this should become a slash command, and also,... it kinda doesn't make much sense rn
/// Print bot's latency to discord.
#[poise::command(prefix_command, check = "crate::checks::check_is_moderator")]
pub async fn latency(prefix_ctx: PrefixCtx<'_>) -> Res<()> {
    let msg_time = prefix_ctx.msg.timestamp;
    let now = Utc::now();
    let latency = now.timestamp_millis() - msg_time.timestamp_millis();
    prefix_ctx
        .msg
        .reply(&prefix_ctx.discord, format!("Latency is **{}ms**", latency))
        .await?;

    Ok(())
}

/// I'm tired,... >.<
#[poise::command(prefix_command, slash_command)]
pub async fn uptime(ctx: Ctx<'_>) -> Res<()> {
    let config = ctx.get_config();
    ctx.send_embed(|e| {
        e.title("Uptime");
        e.description(format!(
            "Started {}",
            util::format_date_detailed(config.time_started)
        ));
    })
    .await?;
    Ok(())
}

/// Send a link to the bot's repository! Feel free contribute!
#[poise::command(prefix_command, slash_command)]
pub async fn repo(ctx: Ctx<'_>) -> Res<()> {
    ctx.say("https://github.com/unixporn/robbb").await?;
    Ok(())
}

/// Get the invite to the unixporn discord server
#[poise::command(prefix_command, slash_command)]
pub async fn invite(ctx: Ctx<'_>) -> Res<()> {
    ctx.say("https://discord.gg/4M7SYzn3BW").await?;
    Ok(())
}

// TODORW include a way to query the description again, probably via fetch -- or a subcommand
/// Set your profiles description.
#[poise::command(prefix_command, guild_only, slash_command)]
pub async fn desc(
    ctx: Ctx<'_>,
    #[description = "Your profile description"] description: Option<String>,
) -> Res<()> {
    let db = ctx.get_db();
    db.set_description(ctx.author().id, description).await?;
    ctx.say_success("Successfully updated your description!")
        .await?;
    Ok(())
}

// TODORW include a way to query the git again, probably via fetch -- or a subcommand
/// Provide a link to your github/gilab/... profile.
#[poise::command(prefix_command, guild_only, slash_command)]
pub async fn git(
    ctx: Ctx<'_>,
    #[description = "Link to your git profile"] link: Option<String>,
) -> Res<()> {
    let db = ctx.get_db();
    db.set_git(ctx.author().id, link).await?;
    ctx.say_success("Successfully updated your git-url!")
        .await?;
    Ok(())
}

// TODORW include a way to query the description again, probably via fetch -- or a subcommand
/// Provide a link to your dotfiles
#[poise::command(prefix_command, guild_only, slash_command, aliases("dots"))]
pub async fn dotfiles(
    ctx: Ctx<'_>,
    #[description = "Link to your dotfiles"] link: Option<String>,
) -> Res<()> {
    let db = ctx.get_db();
    db.set_dotfiles(ctx.author().id, link).await?;
    ctx.say_success("Successfully updated the link to your dotfiles!")
        .await?;
    Ok(())
}
