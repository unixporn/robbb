use crate::{
    extensions::{MessageExt, UserExt},
    log_errors, util, Config,
};

use super::checks::*;
//use super::Config;
use anyhow::{anyhow, Context, Result};
use chrono_humanize::*;
use itertools::Itertools;
use reaction_collector::ReactionAction;
use serenity::framework::standard::CommandGroup;
use serenity::framework::standard::{
    help_commands,
    macros::{group, help},
};
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::framework::standard::{Args, CommandOptions, HelpOptions};
use serenity::{client, collector::reaction_collector, model::prelude::*};
use std::collections::HashSet;

lazy_static::lazy_static! {
    static ref SELECTION_EMOJI: Vec<&'static str> = vec!["1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟"];
}
#[group]
#[only_in(guilds)]
#[commands(restart, mute)]
#[checks(moderator)]
struct Moderator;

#[group]
#[only_in(guilds)]
#[commands(info, modping, pfp, move_users)]
struct General;

#[command]
pub async fn restart(ctx: &client::Context, msg: &Message) -> CommandResult {
    let _ = msg.reply(&ctx, "Shutting down").await;
    ctx.shard.shutdown_clean();

    std::process::exit(1);
}

#[help]
#[individual_command_tip = "If you want more information about a specific command, just pass the command as argument."]
#[command_not_found_text = "Could not find: `{}`."]
#[max_levenshtein_distance(3)]
#[indention_prefix = "+"]
#[lacking_permissions = "Hide"]
#[lacking_role = "Nothing"]
#[wrong_channel = "Strike"]
async fn my_help(
    ctx: &client::Context,
    msg: &Message,
    _args: Args,
    _help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    _owners: HashSet<UserId>,
) -> CommandResult {
    let mut commands = Vec::new();
    for group in groups {
        for command in group.options.commands {
            if help_commands::has_all_requirements(&ctx, command.options, msg).await {
                commands.push(command.options)
            }
        }
    }

    let result = msg
        .channel_id
        .send_message(&ctx, move |m| {
            m.embed(move |e| {
                e.title("Help");
                for command in commands {
                    let command_name = command.names.first().expect("Command had no name");
                    let name = match command.usage {
                        Some(usage) => format!("**{}** - {}", command_name, usage),
                        None => format!("**{}**", command_name),
                    };
                    let description = command.desc.unwrap_or("").to_string();
                    let description = if !command.examples.is_empty() {
                        format!("{}\n{}", description, command.examples.join("\n"))
                    } else {
                        description
                    };
                    e.field(name, description, false);
                }
                e
            })
        })
        .await;
    crate::log_error_value(result);
    Ok(())
}

/// General information over a user.
#[command]
#[usage("info [user]")]
pub async fn info(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = if let Ok(mentioned_user) = args.single::<String>() {
        match disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user).await? {
            Some(mention) => mention,
            None => {
                let _ = msg.reply(&ctx, "Couldn't find anyone with this name :/");
                return Ok(());
            }
        }
    } else {
        msg.author.id
    };
    let member = guild.member(&ctx, mentioned_user_id).await?;

    let created_at = mentioned_user_id.created_at();
    let join_date = member.joined_at.context("Failed to get join date")?;

    let color = member.colour(&ctx).await;

    msg.channel_id
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title(member.user.name_with_disc());
                e.thumbnail(member.user.avatar_or_default());
                if let Some(color) = color {
                    e.color(color);
                }
                e.field("ID/Snowflake", mentioned_user_id.to_string(), false);
                e.field(
                    "Account creation date",
                    util::format_date(created_at),
                    false,
                );
                e.field("Join Date", util::format_date(join_date), false);
                if !member.roles.is_empty() {
                    e.field(
                        "Roles",
                        member.roles.iter().map(|x| x.mention()).join(" "),
                        false,
                    );
                }
                e
            })
        })
        .await?;

    Ok(())
}

/// Show the profile-picture of a user.
#[command]
#[usage("pfp [user]")]
pub async fn pfp(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let mentioned_user_id = if let Ok(mentioned_user) = args.single::<String>() {
        match disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user).await? {
            Some(mention) => mention,
            None => {
                let _ = msg.reply(&ctx, "Couldn't find anyone with this name :/");
                return Ok(());
            }
        }
    } else {
        msg.author.id
    };

    let user = mentioned_user_id.to_user(&ctx).await?;

    let result = msg
        .reply_embed(&ctx, |e| {
            e.title(format!("{}'s profile picture", user.name_with_disc()));
            // TODO embed color
            e.image(user.avatar_or_default());
        })
        .await;
    util::log_error_value(result);
    Ok(())
}

/// Ping all online moderators. Do not abuse!
#[command]
#[usage("modping <reason>")]
pub async fn modping(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    let reason = args.message();
    if reason.trim().is_empty() {
        reply_wrong_usage(ctx, msg, &MODPING_COMMAND_OPTIONS).await;
        return Ok(());
    }

    let guild = msg.guild(&ctx).await.context("Failed to fetch guild")?;
    let mods = guild
        .members
        .values()
        .filter(|member| member.roles.contains(&config.role_mod));

    msg.channel_id
        .send_message(&ctx, |m| {
            m.content(format!(
                "{} pinged moderators {} for reason {}",
                msg.author.mention(),
                mods.map(|m| m.mention()).join(", "),
                reason,
            ))
        })
        .await?;

    Ok(())
}

#[command("move")]
#[usage("move <#channel> [<user> ...]")]
pub async fn move_users(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel = args.single::<ChannelId>()?;
    let rest = args.remains().unwrap_or_default();
    let continuation_msg = channel
        .send_message(&ctx, |m| {
            m.content(format!(
                "{} {}\nContinuation from {}\n({})",
                msg.author.mention(),
                rest,
                msg.channel_id.mention(),
                msg.link()
            ))
        })
        .await?;
    let _ = msg
        .reply_embed(&ctx, |e| {
            e.description(format!(
                "Continued at {}\n{}",
                channel.mention(),
                continuation_msg.link()
            ));
        })
        .await?;
    Ok(())
}

#[command]
#[usage("mute <user> <duration> [reason]")]
pub async fn mute(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let mentioned_user_id = if let Ok(mentioned_user) = args.single::<String>() {
        match disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user).await? {
            Some(mention) => mention,
            None => {
                let _ = msg.reply(&ctx, "Couldn't find anyone with this name :/");
                return Ok(());
            }
        }
    } else {
        reply_wrong_usage(&ctx, &msg, &MUTE_COMMAND_OPTIONS).await;
        return Ok(());
    };

    let duration = match args.single::<humantime::Duration>() {
        Ok(duration) => duration,
        Err(err) => {
            reply_wrong_usage(&ctx, &msg, &MUTE_COMMAND_OPTIONS).await;
            return Ok(());
        }
    };

    let reason = args.single::<String>().unwrap_or_default();

    let guild = msg.guild(&ctx).await.context("Failed to fetch guild")?;
    let mut member = guild.member(&ctx, mentioned_user_id).await?;
    member.add_role(&ctx, config.role_mute).await?;
    // TODO db stuff

    msg.reply(
        &ctx,
        format!("{} has been muted for {}", member.mention(), duration,),
    )
    .await?;
    Ok(())
}

pub async fn disambiguate_user_mention(
    ctx: &client::Context,
    guild: &Guild,
    msg: &Message,
    name: &str,
) -> Result<Option<UserId>> {
    if let Ok(user_id) = name.parse::<UserId>() {
        Ok(Some(user_id))
    } else if let Some(member) =
        async { Some(guild.member(&ctx, name.parse::<u64>().ok()?).await.ok()?) }.await
    {
        Ok(Some(member.user.id))
    } else {
        let member_options = guild
            .members_containing(name, false, true)
            .await
            .into_iter()
            .map(|(mem, _)| mem.clone())
            .collect_vec();

        if member_options.len() == 1 {
            Ok(Some(member_options.first().unwrap().user.id))
        } else {
            Ok(await_reaction_selection(
                &ctx,
                msg.channel_id,
                msg.author.id,
                member_options.clone(),
                "Ambiguous user mention",
                |m| format!("{}", m.mention()),
            )
            .await
            .context("Failed to request user selection")?
            .map(|member| member.user.id))
        }
    }
}

pub async fn await_reaction_selection<'a, T: 'static + Clone + Send + Sync>(
    ctx: &client::Context,
    channel_id: ChannelId,
    by: UserId,
    options: Vec<T>,
    title: &str,
    show: impl Fn(&T) -> String,
) -> Result<Option<T>> {
    if options.is_empty() {
        return Ok(None);
    }
    let options = SELECTION_EMOJI
        .iter()
        .map(|a| a.to_string())
        .zip(options.into_iter())
        .collect_vec();

    let description = options
        .iter()
        .map(|(emoji, value)| format!("{} - {}", emoji, show(&value)))
        .join("\n");

    let selection_message = channel_id
        .send_message(&ctx, |m| {
            m.embed(|e| e.title(title).description(description))
        })
        .await
        .context("Failed to send selection message")?;

    // asynchronously add the reactions
    tokio::spawn({
        let selection_message = selection_message.clone();
        let ctx = ctx.clone();
        let options = options.clone();
        async move {
            for (emoji, _) in options {
                let _ = selection_message
                    .react(&ctx, ReactionType::Unicode(emoji.to_string()))
                    .await;
            }
        }
    });

    let selection = {
        let options = options.clone();
        selection_message
            .await_reaction(&ctx)
            .author_id(by)
            .timeout(std::time::Duration::from_secs(30))
            .filter(move |x| match &x.emoji {
                ReactionType::Unicode(x) => SELECTION_EMOJI[..options.len()].contains(&x.as_str()),
                _ => false,
            })
            .await
    };

    let _ = selection_message.delete(&ctx).await;

    let selection = match selection {
        Some(selection) => selection,
        None => return Ok(None),
    };

    match selection.as_ref() {
        ReactionAction::Added(react) => match &react.emoji {
            ReactionType::Unicode(chosen_emoji) => Ok(options
                .iter()
                .find(|(emoji, _)| emoji == chosen_emoji)
                .map(|(_, x)| x.clone())),
            _ => unreachable!("previously verified in filter"),
        },
        _ => unreachable!("previously verified in filter"),
    }
}

pub async fn reply_wrong_usage(ctx: &client::Context, msg: &Message, opts: &CommandOptions) {
    let error_message = if let Some(usage) = opts.usage {
        format!("Usage: {}", usage)
    } else {
        format!("RTFM, this is not how you use this!")
    };
    let _ = msg.reply(&ctx, error_message).await;
    let _ = msg
        .react(&ctx, ReactionType::Unicode("🤔".to_string()))
        .await;
}
