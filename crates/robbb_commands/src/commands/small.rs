use chrono::Utc;

use super::*;

/// Restart robbb
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn restart(ctx: Ctx<'_>) -> Res<()> {
    let _ = ctx.say_success("Shutting down").await;
    ctx.discord().shard.shutdown_clean();
    std::process::exit(1);
}

/// Make the bot say something. Please don't actually use this :/
#[poise::command(
    slash_command,
    guild_only,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn say(
    ctx: Ctx<'_>,
    #[description = "What you,.. ummmm. I mean _I_ should say"] message: String,
) -> Res<()> {
    ctx.send(|m| m.content("Sure thing!").ephemeral(true))
        .await?;
    ctx.channel_id().say(&ctx.discord(), message).await?;
    Ok(())
}

/// Get some latency information
#[poise::command(
    prefix_command,
    slash_command,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn latency(ctx: Ctx<'_>) -> Res<()> {
    let shard_latency = {
        let shard_manager = ctx.framework().shard_manager.as_ref().lock().await;
        let shard_runners = shard_manager.runners.lock().await;
        shard_runners.values().find_map(|runner| runner.latency)
    };

    let msg_latency = match ctx {
        poise::Context::Application(_) => None,
        poise::Context::Prefix(prefix_ctx) => {
            let msg_time = prefix_ctx.msg.timestamp;
            let now = Utc::now();
            Some(std::time::Duration::from_millis(
                (now.timestamp_millis() - msg_time.timestamp_millis()).abs() as u64,
            ))
        }
    };

    ctx.send_embed(|e| {
        e.title("Latency information");
        if let Some(latency) = shard_latency {
            e.field(
                "Shard latency (last heartbeat send → ACK receive)",
                humantime::Duration::from(latency),
                false,
            );
        }
        if let Some(latency) = msg_latency {
            e.field(
                "Message latency (message timestamp → message received)",
                humantime::Duration::from(latency),
                false,
            );
        }
    })
    .await?;

    Ok(())
}

/// I'm tired,... >.<
#[poise::command(prefix_command, slash_command, category = "Bot-Administration")]
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

// TODORW definitely don't clear when no description is provided, that's super weird

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

//TODORW integrate profile things into the fetch data...

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
