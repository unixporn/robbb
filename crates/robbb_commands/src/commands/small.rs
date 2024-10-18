use anyhow::Context;
use poise::CreateReply;
use robbb_db::fetch_field::FetchField;
use robbb_util::log_error;
use serenity::all::RoleId;

use super::*;

/// Restart robbb
#[poise::command(
    slash_command,
    guild_only,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn restart(ctx: Ctx<'_>) -> Res<()> {
    let _ = ctx.say_success("Shutting down").await;
    ctx.serenity_context().shard.shutdown_clean();
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
    tokio::try_join!(
        ctx.send(CreateReply::default().content("Sure thing!").ephemeral(true)),
        ctx.channel_id().say(ctx.serenity_context(), message),
    )?;
    Ok(())
}

/// Add a role to literally everyone on the server.
#[poise::command(
    slash_command,
    guild_only,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn mass_role(ctx: Ctx<'_>, role: RoleId) -> Res<()> {
    let guild = ctx.guild().context("Not in a guild")?.to_owned();
    let member_count = guild.member_count;
    let mut handled_members = 0;
    let mut last_user = None;

    let progress_msg = ctx.reply(format!("Progress: 0/{}", member_count)).await?;

    loop {
        let members = guild.members(ctx.http(), Some(1000), last_user).await?;

        for member in members.iter() {
            log_error!(
                "Failed to add mass-role to member",
                member.add_role(ctx.http(), role).await
            );
        }
        handled_members += members.len();

        progress_msg
            .edit(
                ctx,
                CreateReply::default()
                    .content(format!("Progress: {handled_members}/{member_count}")),
            )
            .await?;

        if let Some(last) = members.last() {
            last_user = Some(last.user.id);
        } else {
            break;
        }
    }
    Ok(())
}

/// Get some latency information
#[poise::command(
    slash_command,
    category = "Bot-Administration",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn latency(ctx: Ctx<'_>) -> Res<()> {
    let shard_latency = {
        let shard_manager = ctx.framework().shard_manager.as_ref();
        let shard_runners = shard_manager.runners.lock().await;
        shard_runners.values().find_map(|runner| runner.latency)
    };

    ctx.reply_embed_builder(|e| {
        e.title("Latency information").field_opt(
            "Shard latency (last heartbeat send → ACK receive)",
            shard_latency.map(|x| humantime::Duration::from(x).to_string()),
            false,
        )
    })
    .await?;

    Ok(())
}

/// I'm tired,... >.<
#[poise::command(slash_command, category = "Bot-Administration")]
pub async fn uptime(ctx: Ctx<'_>) -> Res<()> {
    let config = ctx.get_config();

    let date = util::format_date_detailed(config.time_started);
    ctx.reply_embed_builder(|e| e.title("Uptime").description(format!("Started {date}"))).await?;
    Ok(())
}

/// Send a link to the bot's repository! Feel free contribute!
#[poise::command(slash_command)]
pub async fn repo(ctx: Ctx<'_>) -> Res<()> {
    ctx.say("https://github.com/unixporn/robbb").await?;
    Ok(())
}

/// Get the invite to the unixporn discord server
#[poise::command(slash_command)]
pub async fn invite(ctx: Ctx<'_>) -> Res<()> {
    ctx.say("https://discord.gg/4M7SYzn3BW").await?;
    Ok(())
}

/// Get a users description. Provide your own using /setfetch.
#[poise::command(guild_only, slash_command)]
pub async fn description(
    ctx: Ctx<'_>,
    #[description = "The user"] user: Option<Member>,
) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let fetch = db.get_fetch(user.user.id).await?;
    if let Some(desc) = fetch.and_then(|x| x.info.get(&FetchField::Description).cloned()) {
        ctx.reply_embed_builder(|e| {
            e.author_user(&user.user).title("Description").description(desc)
        })
        .await?;
    } else {
        ctx.say_error(format!("{} hasn't set their description", user.user.tag())).await?;
    }
    Ok(())
}
/// Get a users dotfiles. Provide your own using /setfetch.
#[poise::command(guild_only, slash_command)]
pub async fn dotfiles(ctx: Ctx<'_>, #[description = "The user"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let fetch = db.get_fetch(user.user.id).await?;
    if let Some(dots) = fetch.and_then(|x| x.info.get(&FetchField::Dotfiles).cloned()) {
        ctx.reply_embed_builder(|e| e.author_user(&user.user).title("Dotfiles").description(dots))
            .await?;
    } else {
        ctx.say_error(format!("{} hasn't provided their dotfiles", user.user.tag())).await?;
    }
    Ok(())
}

/// Get a users git profile. Provide your own using /setfetch.
#[poise::command(guild_only, slash_command)]
pub async fn git(ctx: Ctx<'_>, #[description = "The user"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let db = ctx.get_db();
    let fetch = db.get_fetch(user.user.id).await?;
    if let Some(git) = fetch.and_then(|x| x.info.get(&FetchField::Git).cloned()) {
        ctx.reply_embed_builder(|e| {
            e.author_user(&user.user).title("Git profile").description(git)
        })
        .await?;
    } else {
        ctx.say_error(format!("{} hasn't provided their git profile", user.user.tag())).await?;
    }
    Ok(())
}

/// Get the currently running bot version
#[poise::command(slash_command, guild_only, category = "Bot-Administration", hide_in_help)]
pub async fn version(ctx: Ctx<'_>) -> Res<()> {
    let bot_version = util::BotVersion::get();
    ctx.reply_embed_builder(|e| {
        e.title("Version info")
            .field("profile", bot_version.profile, true)
            .field("commit", bot_version.commit_link(), true)
            .field("message", bot_version.commit_msg, false)
    })
    .await?;

    Ok(())
}

/// Manage application commands (be careful)
///
/// Please only run this when absolutely necessary, as setting up the permissions for the commands again is pain.
#[poise::command(
    slash_command,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    hide_in_help
)]
pub async fn manage_commands(ctx: Ctx<'_>) -> Res<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
