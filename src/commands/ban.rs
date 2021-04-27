use chrono::Duration;

use crate::checks;

use super::*;

/// Ban a user from the server. use `delban` to also delete the messages he sent within the last day.
#[command]
#[usage("ban <@user> <reason>")]
#[aliases("yeet")]
#[only_in(guilds)]
pub async fn ban(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    do_ban(ctx, msg, args, 0).await?;
    Ok(())
}

/// Ban a user from the server, deleting all messages the user sent within the last day.
#[command]
#[only_in(guilds)]
#[usage("delban <@user> <reason>")]
#[aliases("delyeet")]
#[help_available(false)]
#[only_in(guilds)]
pub async fn delban(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    do_ban(ctx, msg, args, 1).await?;
    Ok(())
}

async fn do_ban(
    ctx: &client::Context,
    msg: &Message,
    mut args: Args,
    delete_days: u8,
) -> CommandResult {
    let config = ctx.get_config().await;

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user = &args
        .single::<UserId>()
        .invalid_usage(&BAN_COMMAND_OPTIONS)?;

    let permission_level = checks::get_permission_level(&ctx, &msg).await;
    if permission_level == PermissionLevel::Helper
        && Utc::now().signed_duration_since(mentioned_user.created_at()) > Duration::days(3)
    {
        abort_with!("You can't ban an account older than 3 days");
    }

    let reason = args.remains().invalid_usage(&BAN_COMMAND_OPTIONS)?;

    let user = mentioned_user
        .to_user(&ctx)
        .await
        .context("Failed to retrieve user for banned user")?;

    if reason.to_string().contains("ice") {
        let _ = user
            .dm(&ctx, |m| -> &mut serenity::builder::CreateMessage {
               m.content("Hey ice, you were banned once again. Instead of wasting your time spamming here, please consider seeking help regarding your mental health.\nhttps://www.nimh.nih.gov/health/find-help/index.shtml")
            })
            .await;
    } else {
        let _ = user
            .dm(&ctx, |m| {
                m.embed(|e| {
                    e.title(format!("You were banned from {}", guild.name));
                    e.field("Reason", reason, false)
                })
            })
            .await;
    }
    guild
        .ban_with_reason(&ctx, mentioned_user, delete_days, reason)
        .await?;

    config
        .log_bot_action(&ctx, |e| {
            e.title("User yote");
            e.author(|a| a.name(msg.author.tag()).icon_url(msg.author.face()));
            e.description(format!(
                "{} ({}) has been yote\n{}",
                user.mention(),
                user.tag(),
                msg.to_context_link(),
            ));
            e.field("Reason", reason, false);
        })
        .await;

    msg.reply_success_mod_action(&ctx, "Successfully yote!")
        .await?;
    Ok(())
}
