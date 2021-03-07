use crate::{
    db::Db,
    error_out,
    extensions::{MessageExt, UserExt},
    util, Config,
};

use super::checks::*;
//use super::Config;
use anyhow::{Context, Result};
use chrono::Utc;
use itertools::Itertools;
use reaction_collector::ReactionAction;
use serenity::{
    client,
    collector::reaction_collector,
    framework::standard::{
        help_commands,
        macros::{command, group, help},
        Args, CommandGroup, CommandOptions, CommandResult, HelpOptions,
    },
    model::prelude::*,
};
use std::collections::HashSet;
use thiserror::Error;

pub mod ban;
pub mod errors;
pub mod help;
pub mod info;
pub mod modping;
pub mod move_users;
pub mod mute;
pub mod note;
pub mod pfp;
pub mod small;
pub mod warn;
use ban::*;
pub use errors::*;
pub use help::*;
use info::*;
use modping::*;
use move_users::*;
use mute::*;
use note::*;
use pfp::*;
use small::*;
use warn::*;

lazy_static::lazy_static! {
    static ref SELECTION_EMOJI: Vec<&'static str> = vec!["1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟"];
}

#[group]
#[only_in(guilds)]
#[commands(restart, mute, warn, note, notes, latency, say, ban, delban)]
#[checks(moderator)]
struct Moderator;

#[group]
#[only_in(guilds)]
#[commands(info, modping, pfp, move_users, repo)]
struct General;

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
                &msg,
                msg.author.id,
                member_options.clone(),
                "Ambiguous user mention",
                |m| format!("{} ({})", m.mention(), m.user.name_with_disc()),
            )
            .await
            .context("Failed to request user selection")?
            .map(|member| member.user.id))
        }
    }
}

pub async fn await_reaction_selection<'a, T: 'static + Clone + Send + Sync>(
    ctx: &client::Context,
    replying_to: &Message,
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

    let selection_message = replying_to
        .reply_embed(&ctx, |e| {
            e.title(title).description(description);
        })
        .await
        .context("Failed to send selection message")?;

    react_async(
        &ctx,
        &selection_message,
        options
            .iter()
            .map(|(emoji, _)| ReactionType::Unicode(emoji.to_string()))
            .collect_vec(),
    );

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

pub fn react_async(ctx: &client::Context, msg: &Message, reactions: Vec<ReactionType>) {
    let msg = msg.clone();
    let ctx = ctx.clone();
    tokio::spawn(async move {
        for emoji in reactions {
            let _ = msg.react(&ctx, emoji).await;
        }
    });
}
