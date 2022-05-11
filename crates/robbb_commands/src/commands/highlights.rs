use poise::serenity_prelude::CreateEmbed;

use super::*;
use crate::checks::{self, PermissionLevel};

pub fn highlights_commands() -> Command<UserData, Error> {
    Command {
        subcommands: vec![
            highlights_add(),
            highlights_list(),
            highlights_clear(),
            highlights_remove(),
        ],
        ..highlights()
    }
}

/// Get notified when someone mentions a word you care about.
#[poise::command(
    slash_command,
    category = "Miscellaneous",
    rename = "highlight",
    aliases("highlights", "hl")
)]
pub async fn highlights(_: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Add a new highlight
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    category = "Miscellaneous",
    rename = "add"
)]
pub async fn highlights_add(
    ctx: Ctx<'_>,
    #[description = "The word you want to be notified about"] trigger: String,
) -> Res<()> {
    if trigger.len() < 3 {
        abort_with!("Highlight has to be longer than 2 characters");
    }

    let db = ctx.get_db();
    let max_highlight_cnt = match checks::get_permission_level(ctx).await {
        PermissionLevel::Mod => 20,
        _ => 4,
    };

    let highlights = db.get_highlights().await?;
    let highlights_by_user_cnt = highlights.triggers_for_user(ctx.author().id).count();

    if highlights_by_user_cnt >= max_highlight_cnt {
        abort_with!(UserErr::Other(format!(
            "Sorry, you can only watch a maximum of {} highlights",
            max_highlight_cnt
        )));
    }

    ctx.author()
        .id
        .create_dm_channel(&ctx.discord())
        .await
        .user_error("Couldn't open a DM to you - do you have me blocked?")?
        .send_message(&ctx.discord(), |m| {
            m.embed(|e| {
                e.title("Test to see if you can receive DMs");
                e.description(format!(
                    "If everything went ok, you'll be notified whenever someone says `{}`",
                    trigger
                ))
            })
        })
        .await
        .user_error("Couldn't send you a DM :/\nDo you allow DMs from server members?")?;

    db.set_highlight(ctx.author().id, trigger.clone())
        .await
        .user_error(
            "Couldn't add highlight, something went wrong (highlight might already be present)",
        )?;

    ctx.say_success(format!(
        "You will be notified whenever someone says {}",
        trigger
    ))
    .await?;

    Ok(())
}

/// List all of your highlights
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    category = "Miscellaneous",
    rename = "list"
)]
pub async fn highlights_list(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();
    let highlights = db.get_highlights().await?;

    let highlights_list = highlights.triggers_for_user(ctx.author().id).join("\n");

    if highlights_list.is_empty() {
        abort_with!("You don't seem to have set any highlights");
    } else {
        try_dm_or_ephemeral_response(ctx, |e| {
            e.title("Your highlights");
            e.description(highlights_list);
        })
        .await?;
    }
    Ok(())
}

/// Remove a highlight
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    category = "Miscellaneous",
    rename = "remove"
)]
pub async fn highlights_remove(
    ctx: Ctx<'_>,
    #[autocomplete = "autocomplete_highlights"]
    #[description = "Which highlight do you want to remove"]
    trigger: String,
) -> Res<()> {
    let db = ctx.get_db();
    db.remove_highlight(ctx.author().id, trigger.clone())
        .await
        .user_error("Failed to remove the highlight.")?;
    ctx.say_success(format!(
        "You will no longer be notified when someone says '{}'",
        trigger
    ))
    .await?;
    Ok(())
}

/// Remove all of your highlights
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    category = "Miscellaneous",
    rename = "clear"
)]
pub async fn highlights_clear(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();
    db.rm_highlights_of(ctx.author().id).await?;
    ctx.say_success("Your highlights have been successfully cleared.")
        .await?;
    Ok(())
}

/// When in an `ApplicationContext`, send the reply as a ephemeral message.
/// When in a `PrefixContext`, attempt to send the message in DMs,
/// and give a generic error message when the user doesn't allow for that.
async fn try_dm_or_ephemeral_response(
    ctx: Ctx<'_>,
    build: impl FnOnce(&mut CreateEmbed) + Send + Sync,
) -> Res<()> {
    match ctx {
        poise::Context::Application(_) => {
            ctx.send_embed_full(true, |e| {
                build(e);
            })
            .await?;
        }
        poise::Context::Prefix(_) => {
            ctx.author()
                .id
                .create_dm_channel(&ctx.discord())
                .await
                .user_error("Couldn't open a DM to you - do you have me blocked?")?
                .send_message(&ctx.discord(), |m| {
                    m.embed(|e| {
                        build(e);
                        e
                    })
                })
                .await
                .user_error("Couldn't send you a DM :/\nDo you allow DMs from server members?")?;
        }
    }
    Ok(())
}

async fn autocomplete_highlights(ctx: Ctx<'_>, partial: String) -> Vec<String> {
    let db = ctx.get_db();
    if let Ok(highlights) = db.get_highlights().await {
        highlights
            .triggers_for_user(ctx.author().id)
            .filter(|x| x.contains(&partial))
            .map(|x| x.to_string())
            .collect_vec()
    } else {
        Vec::new()
    }
}
