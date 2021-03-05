use super::checks::*;
use super::Config;
use anyhow::{anyhow, Context, Result};
use chrono_humanize::*;
use itertools::Itertools;
use reaction_collector::ReactionAction;
use serenity::builder::CreateMessage;
use serenity::framework::standard::macros::{check, group, help};
use serenity::framework::standard::{
    help_commands, Args, CommandError, CommandGroup, CommandOptions, HelpOptions, Reason,
};
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::utils::EmbedMessageBuilding;
use serenity::utils::MessageBuilder;
use serenity::{
    client,
    collector::{self, reaction_collector},
};
use std::collections::HashSet;

lazy_static::lazy_static! {
    static ref SELECTION_EMOJI: Vec<&'static str> = vec!["1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟"];
}
#[group]
#[only_in(guilds)]
#[commands(restart)]
#[checks(moderator)]
struct Moderator;

#[group]
#[only_in(guilds)]
#[commands(info, modping)]
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
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    // let _ = help_commands::with_embeds(ctx, msg, args, help_options, groups, owners).await;

    let commands: Vec<&CommandOptions> = groups
        .iter()
        .flat_map(|group| group.options.commands.iter().map(|cmd| cmd.options))
        .collect_vec();

    let result = msg
        .channel_id
        .send_message(&ctx, move |m| {
            m.embed(move |e| {
                e.title("Help");
                for command in commands {
                    let command_name = command.names.first().expect("Command had no name");
                    let name = if let Some(usage) = command.usage {
                        format!("**{}** - {}", command_name, usage)
                    } else {
                        format!("**{}**", command_name)
                    };
                    e.field(name, command.desc.unwrap_or("No description"), false);
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
        disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user).await?
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
                e.title(format!(
                    "{}#{}",
                    member.user.name, member.user.discriminator
                ));
                e.thumbnail(
                    member
                        .user
                        .avatar_url()
                        .unwrap_or(member.user.default_avatar_url()),
                );
                if let Some(color) = color {
                    e.color(color);
                }
                e.field("ID/Snowflake", mentioned_user_id.to_string(), false);
                e.field(
                    "Account creation date",
                    HumanTime::from(created_at).to_text_en(Accuracy::Precise, Tense::Present),
                    false,
                );
                e.field(
                    "Join Date",
                    HumanTime::from(join_date).to_text_en(Accuracy::Precise, Tense::Present),
                    false,
                );
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

    // let mods = {
    //     let mut members: Vec<Member> = Vec::new();
    //     loop {
    //         let mut new_members = guild
    //             .members(&ctx, Some(1000), members.last().map(|m| m.user.id))
    //             .await?;
    //         if new_members.is_empty() {
    //             members.append(&mut new_members);
    //             break;
    //         }
    //         members.append(&mut new_members);
    //     }
    //     members
    //         .into_iter()
    //         .filter(|member| member.roles.contains(&config.role_mod))
    //         .collect_vec()
    // };
    //
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

pub async fn disambiguate_user_mention(
    ctx: &client::Context,
    guild: &Guild,
    msg: &Message,
    name: &str,
) -> Result<UserId> {
    if let Ok(user_id) = name.parse::<UserId>() {
        return Ok(user_id);
    }
    if let Some(member) =
        async { Some(guild.member(&ctx, name.parse::<u64>().ok()?).await.ok()?) }.await
    {
        return Ok(member.user.id);
    }

    let member_options = guild
        .members_containing(name, false, true)
        .await
        .into_iter()
        .map(|(mem, _)| mem.clone())
        .collect_vec();

    if member_options.is_empty() {
        let _ = msg
            .reply(&ctx, "Couldn't find anyone with this name :/")
            .await;
        return Err(anyhow!("No user found"));
    } else if member_options.len() == 1 {
        return Ok(member_options.first().unwrap().user.id);
    }

    await_reaction_selection(
        &ctx,
        msg.channel_id,
        msg.author.id,
        member_options.clone(),
        "Ambiguous user mention".to_string(),
        |m| format!("{}", m.mention()),
    )
    .await
    .context("Failed to request selection")?
    .context("Nothing selected")
    .map(|x| x.user.id)
}

pub async fn await_reaction_selection<'a, T: 'static + Clone + Send + Sync>(
    ctx: &client::Context,
    channel_id: ChannelId,
    by: UserId,
    options: Vec<T>,
    title: String,
    show: impl Fn(&T) -> String,
) -> Result<Option<T>> {
    let options = SELECTION_EMOJI
        .iter()
        .zip(options.into_iter())
        .map(|(a, b)| (a.to_string(), b))
        .collect_vec();
    let msg = channel_id
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title(title);
                e.description(
                    options
                        .iter()
                        .map(|(emoji, value)| format!("{} - {}", emoji, show(&value)))
                        .join("\n"),
                )
            });
            m.reactions(
                options
                    .iter()
                    .map(|(react, _)| ReactionType::Unicode(react.to_string())),
            )
        })
        .await
        .context("Failed to send selection message")?;

    let selection = {
        let options = options.clone();
        msg.await_reaction(&ctx)
            .author_id(by)
            .timeout(std::time::Duration::from_secs(30))
            .filter(move |x| match &x.emoji {
                ReactionType::Unicode(x) => SELECTION_EMOJI[..options.len()].contains(&x.as_str()),
                _ => false,
            })
            .await
    };

    let _ = msg.delete(&ctx).await;

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
