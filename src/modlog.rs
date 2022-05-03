use itertools::Itertools;
use poise::serenity_prelude::Message;
use serenity::{client, model::prelude::User, prelude::Mentionable};

use crate::{
    db::mute::Mute,
    extensions::{ClientContextExt, CreateEmbedExt, MessageExt, PoiseContextExt, UserExt},
    prelude::Ctx,
    util,
};

pub async fn log_note(ctx: Ctx<'_>, user: &User, note_content: &str) {
    let config = ctx.data().config.clone();

    config
        .log_bot_action(&ctx.discord(), |e| {
            e.title("Note");
            e.author_user(ctx.author().clone());
            e.thumbnail(user.face());
            e.description(format!(
                "{} took a note about {}",
                ctx.author().id.mention(),
                user.mention_and_tag(),
            ));
            e.field("Note", note_content, false);
        })
        .await;
}
pub async fn log_warn(
    ctx: &Ctx<'_>,
    context_msg: &Message,
    user: User,
    warn_count: i32,
    reason: &str,
) {
    let config = ctx.get_config();
    config
        .log_bot_action(&ctx.discord(), |e| {
            e.title("Warn");
            e.author_user(ctx.author().clone());
            e.thumbnail(user.face());
            e.description(format!(
                "{} was warned by {} _({} warn)_\n{}",
                user.mention_and_tag(),
                ctx.author().id.mention(),
                util::format_count(warn_count),
                context_msg.to_context_link(),
            ));
            e.field("Reason", reason, false);
        })
        .await;
}

pub async fn log_kick(ctx: Ctx<'_>, context_msg: &Message, user: User, reason: &str) {
    let config = ctx.get_config();
    config
        .log_bot_action(&ctx.discord(), |e| {
            e.title("Kick");
            e.thumbnail(user.face());
            e.author_user(ctx.author().clone());
            e.description(format!(
                "User {} was kicked by {}\n{}",
                user.mention_and_tag(),
                ctx.author().id.mention(),
                context_msg.to_context_link()
            ));
            e.field("Reason", reason, false);
        })
        .await;
}

pub async fn log_ban(ctx: Ctx<'_>, context_msg: &Message, successful_bans: &[User], reason: &str) {
    let config = ctx.get_config();
    config
        .log_bot_action(&ctx.discord(), |e| {
            e.title("Ban");
            e.author_user(ctx.author().clone());
            e.description(format!(
                "yote user(s):\n{}\n{}",
                successful_bans
                    .iter()
                    .map(|x| format!("- {}", x.mention_and_tag()))
                    .join("\n"),
                context_msg.to_context_link(),
            ));
            e.field("Reason", reason, false);
        })
        .await;
}

pub async fn log_unban(ctx: Ctx<'_>, user: User) {
    let config = ctx.get_config();
    config
        .log_bot_action(&ctx.discord(), |e| {
            e.title("Unban");
            e.author_user(ctx.author().clone());
            e.thumbnail(user.face());
            e.description(format!("{} has been deyote", user.mention_and_tag()));
        })
        .await;
}

pub async fn log_mute(
    ctx: &Ctx<'_>,
    context_msg: &Message,
    user: &User,
    duration: humantime::Duration,
    reason: Option<String>,
) {
    let config = ctx.get_config();

    let end_time = chrono::Duration::from_std(duration.into())
        .ok()
        .and_then(|duration| chrono::Utc::now().checked_add_signed(duration))
        .map(util::format_date_detailed);

    config
        .log_bot_action(ctx.discord(), |e| {
            e.title("Mute");
            e.author_user(ctx.author().clone());
            e.thumbnail(user.face());
            e.description(format!(
                "User {} ({}) was muted by {}\n{}",
                user.id.mention(),
                user.tag(),
                ctx.author().id.mention(),
                context_msg.to_context_link(),
            ));
            e.field("Duration", format!("{}", duration), false);
            end_time.map(|t| e.field("End", t, false));
            reason.map(|r| e.field("Reason", r, false));
        })
        .await;
}

pub async fn log_mute_for_spamming(
    ctx: &client::Context,
    spam_msg: &Message,
    duration: std::time::Duration,
) {
    let config = ctx.get_config().await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Automute");
            e.thumbnail(spam_msg.author.face());
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

pub async fn log_user_mute_ended(ctx: &client::Context, mute: &Mute) {
    let config = ctx.get_config().await;
    let user = mute.user.to_user(&ctx).await;
    config
        .log_bot_action(&ctx, |e| {
            e.title("Mute ended");
            if let Ok(user) = user {
                e.description(format!("{} is now unmuted", user.mention_and_tag()));
                e.thumbnail(user.face());
            } else {
                e.description(format!("{} is now unmuted", mute.user.mention()));
            };
        })
        .await;
}
