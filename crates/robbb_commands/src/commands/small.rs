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

/// Get a users description
#[poise::command(
    guild_only,
    slash_command,
    subcommands("desc_set", "desc_get", "desc_clear"),
    rename = "description"
)]
pub async fn desc(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Clear your description
#[poise::command(prefix_command, guild_only, slash_command, rename = "clear")]
pub async fn desc_clear(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();
    db.set_description(ctx.author().id, None).await?;
    ctx.say_success("Successfully cleared your description!")
        .await?;
    Ok(())
}

/// Set your profiles description.
#[poise::command(prefix_command, guild_only, slash_command, rename = "set")]
pub async fn desc_set(
    ctx: Ctx<'_>,
    #[description = "Your profile description"] description: String,
) -> Res<()> {
    let db = ctx.get_db();
    if description.len() < 200 {
        db.set_description(ctx.author().id, Some(description))
            .await?;
        ctx.say_success("Successfully updated your description!")
            .await?;
    } else {
        ctx.say_error("Description may not be longer than 200 characters")
            .await?;
    }
    Ok(())
}

/// Get a users description
#[poise::command(prefix_command, guild_only, slash_command, rename = "get")]
pub async fn desc_get(ctx: Ctx<'_>, #[description = "The user"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let profile = db.get_profile(user.user.id).await?;
    if let Some(desc) = profile.description {
        ctx.send_embed(|e| {
            e.author_user(&user.user);
            e.title("Description");
            e.description(desc);
        })
        .await?;
    } else {
        ctx.say_error(format!("{} hasn't set their description", user.user.tag()))
            .await?;
    }
    Ok(())
}

/// Link to your dotfiles
#[poise::command(
    guild_only,
    slash_command,
    subcommands("dotfiles_set", "dotfiles_get", "dotfiles_clear"),
    rename = "dotfiles"
)]
pub async fn dotfiles(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Clear your dotfiles
#[poise::command(prefix_command, guild_only, slash_command, rename = "clear")]
pub async fn dotfiles_clear(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();
    db.set_dotfiles(ctx.author().id, None).await?;
    ctx.say_success("Successfully cleared your dotfiles!")
        .await?;
    Ok(())
}

/// Provide a link to your dotfiles
#[poise::command(prefix_command, guild_only, slash_command, rename = "set")]
pub async fn dotfiles_set(
    ctx: Ctx<'_>,
    #[description = "Link to your dotfiles"] link: String,
) -> Res<()> {
    let db = ctx.get_db();
    if util::validate_url(&link) {
        db.set_dotfiles(ctx.author().id, Some(link)).await?;
        ctx.say_success("Successfully updated the link to your dotfiles!")
            .await?;
    } else {
        ctx.say_error("Dotfiles must be a valid link").await?;
    }
    Ok(())
}

/// Get a users dotfiles
#[poise::command(prefix_command, guild_only, slash_command, rename = "get")]
pub async fn dotfiles_get(
    ctx: Ctx<'_>,
    #[description = "The user"] user: Option<Member>,
) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let profile = db.get_profile(user.user.id).await?;
    if let Some(dots) = profile.dotfiles {
        ctx.send_embed(|e| {
            e.author_user(&user.user);
            e.title("Dotfiles");
            e.description(dots);
        })
        .await?;
    } else {
        ctx.say_error(format!(
            "{} hasn't provided their dotfiles",
            user.user.tag()
        ))
        .await?;
    }
    Ok(())
}

/// Link to your git profile
#[poise::command(
    guild_only,
    slash_command,
    subcommands("git_set", "git_clear", "git_get"),
    rename = "git"
)]
pub async fn git(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Clear your git profile
#[poise::command(prefix_command, guild_only, slash_command, rename = "clear")]
pub async fn git_clear(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();
    db.set_git(ctx.author().id, None).await?;
    ctx.say_success("Successfully cleared your git profile!")
        .await?;
    Ok(())
}

/// Provide a link to your git profile
#[poise::command(prefix_command, guild_only, slash_command, rename = "set")]
pub async fn git_set(
    ctx: Ctx<'_>,
    #[description = "Link to your git profile"] link: String,
) -> Res<()> {
    let db = ctx.get_db();
    if util::validate_url(&link) {
        db.set_git(ctx.author().id, Some(link)).await?;
        ctx.say_success("Successfully updated the link to your git profile!")
            .await?;
    } else {
        ctx.say_error("Git profile must be a valid link").await?;
    }
    Ok(())
}

/// Get a users git profile
#[poise::command(prefix_command, guild_only, slash_command, rename = "get")]
pub async fn git_get(ctx: Ctx<'_>, #[description = "The user"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let profile = db.get_profile(user.user.id).await?;
    if let Some(git) = profile.git {
        ctx.send_embed(|e| {
            e.author_user(&user.user);
            e.title("Git profile");
            e.description(git);
        })
        .await?;
    } else {
        ctx.say_error(format!(
            "{} hasn't provided their git profile",
            user.user.tag()
        ))
        .await?;
    }
    Ok(())
}
