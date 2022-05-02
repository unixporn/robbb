use itertools::Itertools;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::{channel::Message, prelude::User},
    prelude::Mentionable,
};

use crate::{
    db::mute::Mute,
    extensions::{ClientContextExt, MessageExt, UserExt},
    util,
};

pub async fn log_note(ctx: &Context, command_msg: &Message, user: &User, note_content: &str) {
    let config = ctx.get_config().await;

    config
        .log_bot_action(&ctx, |e| {
            e.title("Note");
            set_author_section(e, &command_msg.author);
            e.description(format!(
                "{} took a note about {}",
                command_msg.author.id.mention(),
                user.mention_and_tag(),
            ));
            e.field("Note", note_content, false);
        })
        .await;
}

pub async fn log_warn(
    ctx: &Context,
    command_msg: &Message,
    warned_user: User,
    warn_count: i32,
    reason: &str,
) {
    let config = ctx.get_config().await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Warn");
            set_author_section(e, &command_msg.author);
            e.description(format!(
                "{} was warned by {} _({} warn)_\n{}",
                warned_user.mention_and_tag(),
                command_msg.author.id.mention(),
                util::format_count(warn_count),
                command_msg.to_context_link(),
            ));
            e.field("Reason", reason, false);
        })
        .await;
}

pub async fn log_kick(ctx: &Context, command_msg: &Message, kicked_user: User, reason: &str) {
    let config = ctx.get_config().await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Kick");
            set_author_section(e, &command_msg.author);
            e.description(format!(
                "User {} was kicked by {}\n{}",
                kicked_user.mention_and_tag(),
                command_msg.author.id.mention(),
                command_msg.to_context_link()
            ));
            e.field("Reason", reason, false);
        })
        .await;
}

pub async fn log_ban(ctx: &Context, command_msg: &Message, successful_bans: &[User], reason: &str) {
    let config = ctx.get_config().await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Ban");
            set_author_section(e, &command_msg.author);
            e.description(format!(
                "yote user(s):\n{}\n{}",
                successful_bans
                    .iter()
                    .map(|x| format!("- {}", x.mention_and_tag()))
                    .join("\n"),
                command_msg.to_context_link(),
            ));
            e.field("Reason", reason, false);
        })
        .await;
}

pub async fn log_unban(ctx: &Context, command_msg: &Message, unbanned_user: User) {
    let config = ctx.get_config().await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Unban");
            set_author_section(e, &command_msg.author);
            e.description(format!(
                "{} has been deyote",
                unbanned_user.mention_and_tag()
            ));
        })
        .await;
}

pub async fn log_mute(
    ctx: &Context,
    command_msg: &Message,
    mentioned_user: &User,
    duration: humantime::Duration,
    reason: Option<&str>,
) {
    let config = ctx.get_config().await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Mute");
            set_author_section(e, &command_msg.author);
            e.description(format!(
                "User {} ({}) was muted by {}\n{}",
                mentioned_user.id.mention(),
                mentioned_user.tag(),
                command_msg.author.id.mention(),
                command_msg.to_context_link(),
            ));
            e.field("Duration", duration, false);
            reason.map(|r| e.field("Reason", r, false));
        })
        .await;
}

pub async fn log_mute_for_spamming(
    ctx: &Context,
    spam_msg: &Message,
    duration: std::time::Duration,
) {
    let config = ctx.get_config().await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Automute");
            e.description(format!(
                "User {} was muted for spamming\n{}",
                spam_msg.author.mention_and_tag(),
                spam_msg.to_context_link(),
            ));
            e.field(
                "Duration",
                humantime::Duration::from(duration).to_string(),
                false,
            );
        })
        .await;
}

pub async fn log_user_mute_ended(ctx: &Context, mute: &Mute) {
    let config = ctx.get_config().await;
    let user = mute.user.to_user(&ctx).await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Mute ended");
            e.description(format!(
                "{} is now unmuted",
                if let Ok(user) = user {
                    user.mention_and_tag()
                } else {
                    mute.user.mention().to_string()
                }
            ));
        })
        .await;
}

fn set_author_section(e: &mut CreateEmbed, author: &User) {
    e.author(|a| {
        a.name(author.tag())
            .icon_url(author.face())
            .url(format!("https://discord.com/users/{}", author.id))
    });
}
