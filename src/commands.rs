use super::checks::*;
use super::Config;
use anyhow::{anyhow, Context, Result};
use chrono_humanize::*;
use itertools::Itertools;
use serenity::client;
use serenity::framework::standard::macros::{check, group, help};
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandOptions, HelpOptions, Reason,
};
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::utils::EmbedMessageBuilding;
use serenity::utils::MessageBuilder;
use std::collections::HashSet;

#[group]
#[only_in(guilds)]
#[commands(restart, uthere)]
#[checks(moderator)]
struct Moderator;

#[group]
#[only_in(guilds)]
#[commands(info)]
struct General;

#[command]
pub async fn restart(ctx: &client::Context, msg: &Message) -> CommandResult {
    let _ = msg.reply(&ctx, "Shutting down").await;
    ctx.shard.shutdown_clean();

    std::process::exit(1);
}

#[command]
pub async fn uthere(ctx: &client::Context, msg: &Message) -> CommandResult {
    let _ = msg.reply(&ctx, "Ye, I'm here!").await;
    Ok(())
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
    context: &client::Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

/// General information over a user.
#[command]
#[only_in("guild")]
pub async fn info(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let mentioned_user_id = args.single::<UserId>().unwrap_or(msg.author.id);
    let member = msg
        .guild(&ctx)
        .await
        .context("Failed to load guild")?
        .member(&ctx, mentioned_user_id)
        .await?;

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
