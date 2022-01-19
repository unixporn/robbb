use chrono::Duration;

use crate::checks;

use super::*;

/// Ban one or more users from the server. use `delban` to also delete the messages he sent within the last day.
#[command]
#[usage("ban <@user>[,<@user>,<@user>,...] <reason>")]
#[aliases("yeet")]
#[only_in(guilds)]
pub async fn ban(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    do_ban(ctx, msg, args, 0).await?;
    Ok(())
}

/// Ban one or more users from the server, deleting all messages the user sent within the last day.
#[command]
#[only_in(guilds)]
#[usage("delban <@user>[,<@user>,<@user>,...] <reason>")]
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

    let guild = msg.guild(&ctx).context("Failed to load guild")?;

    let mentioned_users = &args
        .single::<String>()
        .invalid_usage(&BAN_COMMAND_OPTIONS)?
        .split(',')
        .map(|x| {
            x.parse::<UserId>()
                .with_user_error(|_| format!("{} is not a valid user id", x))
        })
        .collect::<Result<Vec<UserId>, UserErr>>()?;

    let reason = args.remains().invalid_usage(&BAN_COMMAND_OPTIONS)?;
    let reason_has_user_id = reason
        .split(' ')
        .next()
        .map(|x| x.parse::<UserId>().is_ok())
        == Some(true);
    if reason_has_user_id {
        abort_with!("Found a user id in your ban reason. Make sure to list the users you want to ban separated by just a comma.");
    }

    let mut disallowed_bans = Vec::new();
    let mut not_found_users = Vec::new();
    let mut successful_bans = Vec::new();

    let permission_level = checks::get_permission_level(&ctx, &msg).await;

    for user in mentioned_users {
        match handle_single_ban(&ctx, &guild, permission_level, *user, reason, delete_days).await {
            std::result::Result::Ok(user) => {
                successful_bans.push(user);
            }
            std::result::Result::Err(BanFailedReason::HelperRestriction(user)) => {
                disallowed_bans.push(user);
            }
            std::result::Result::Err(BanFailedReason::UserNotFound) => {
                not_found_users.push(user);
            }
            std::result::Result::Err(BanFailedReason::Other(err)) => {
                tracing::error!("{}", err);
                let _ = msg
                    .reply_error(
                        &ctx,
                        format!("Something went wrong banning {} ({})", user, user.mention()),
                    )
                    .await;
            }
        }
    }

    if !not_found_users.is_empty() {
        let _ = msg
            .reply_error(
                &ctx,
                format!(
                    "The following users don't seem to exist:\n{}",
                    not_found_users
                        .into_iter()
                        .map(|x| format!("- {}", x.mention()))
                        .join("\n")
                ),
            )
            .await;
    }

    if !disallowed_bans.is_empty() {
        let _ = msg.reply_error(&ctx,
            format!(
                "Failed to ban the following users because of the 3 day account / join age restriction for helpers:\n{}", 
                disallowed_bans.into_iter().map(|x| format!("- {} ({})", x.tag(), x.id)).join("\n")
            )
        ).await;
    }

    if !successful_bans.is_empty() {
        let _ = msg
            .reply_success_mod_action(
                &ctx,
                format!(
                    "successfully yote\n{}",
                    successful_bans
                        .iter()
                        .map(|x| format!("- {} ({})", x.tag(), x.id))
                        .join("\n")
                ),
            )
            .await;

        config
            .log_bot_action(&ctx, |e| {
                e.title("User yote");
                e.author(|a| a.name(msg.author.tag()).icon_url(msg.author.face()));
                e.description(format!(
                    "yote user(s):\n{}\n{}",
                    successful_bans
                        .iter()
                        .map(|x| format!("- {} ({})", x.mention(), x.tag()))
                        .join("\n"),
                    msg.to_context_link(),
                ));
                e.field("Reason", reason, false);
            })
            .await;
    }

    Ok(())
}

enum BanFailedReason {
    HelperRestriction(User),
    UserNotFound,
    Other(anyhow::Error),
}
impl From<anyhow::Error> for BanFailedReason {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e)
    }
}

async fn handle_single_ban(
    ctx: &client::Context,
    guild: &Guild,
    permission_level: PermissionLevel,
    user: UserId,
    reason: &str,
    delete_days: u8,
) -> Result<User, BanFailedReason> {
    let user = user
        .to_user(&ctx)
        .await
        .map_err(|_| BanFailedReason::UserNotFound)?;

    let ban_allowed = if permission_level == PermissionLevel::Helper {
        let member = guild.member(&ctx, user.id).await;
        let join_or_create_date = member
            .ok()
            .and_then(|x| x.joined_at)
            .unwrap_or_else(|| user.created_at());
        Utc::now().signed_duration_since(join_or_create_date) < Duration::days(3)
    } else {
        permission_level == PermissionLevel::Mod
    };

    if !ban_allowed {
        return Err(BanFailedReason::HelperRestriction(user));
    }

    if reason.to_string().contains("ice") {
        let _ = user
            .dm(&ctx, |m| -> &mut serenity::builder::CreateMessage {
               m.content(indoc::formatdoc!(
                   "{}

                   Hey ice, you were banned once again. 
                   Instead of wasting your time spamming here, please consider seeking help regarding your mental health.
                   https://www.nimh.nih.gov/health/find-help/index.shtml",
                   reason.replacen("ice", "", 1),
               ))
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
        .ban_with_reason(&ctx, &user, delete_days, reason)
        .await
        .context("Ban failed")?;

    Ok(user)
}
