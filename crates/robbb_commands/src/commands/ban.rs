use anyhow::Context;
use chrono::{Duration, Utc};
use poise::serenity_prelude::User;

use crate::checks::{self, PermissionLevel};

use super::*;

/// Ban a user from the server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator"
)]
pub async fn ban(
    ctx: Ctx<'_>,
    #[description = "Who is the criminal?"]
    #[rename = "criminal"]
    user: User,
    #[description = "Should I delete their recent messages?"]
    #[rename = "delete"]
    #[flag]
    delete_messages: bool,
    #[description = "What did they do?"] reason: String,
) -> Res<()> {
    do_ban(ctx, vec![user], reason, if delete_messages { 1 } else { 0 }).await?;
    Ok(())
}

/// Ban multiple users from the server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator"
)]
pub async fn ban_many(
    ctx: Ctx<'_>,
    #[description = "List of user-ids, separated by commas"]
    #[rename = "criminals"]
    user_ids: String,
    #[description = "Should I delete their recent messages?"]
    #[rename = "delete"]
    #[flag]
    delete_messages: bool,
    #[description = "What did they do?"] reason: String,
) -> Res<()> {
    let mut users = Vec::new();
    for user_id in user_ids.split(',') {
        let user_id = user_id
            .trim()
            .parse::<UserId>()
            .with_user_error(|_| format!("{} is not a valid user id", user_id))?
            .to_user(&ctx.discord())
            .await?;
        users.push(user_id);
    }

    do_ban(ctx, users, reason, if delete_messages { 1 } else { 0 }).await?;
    Ok(())
}

async fn do_ban(ctx: Ctx<'_>, users: Vec<User>, reason: String, delete_days: u8) -> Res<()> {
    let guild = ctx.guild().context("Failed to load guild")?;

    let mut disallowed_bans = Vec::new();
    let mut successful_bans = Vec::new();

    let permission_level = checks::get_permission_level(ctx).await;

    for user in users {
        match handle_single_ban(
            ctx,
            &guild,
            permission_level,
            user.clone(),
            &reason,
            delete_days,
        )
        .await
        {
            std::result::Result::Ok(user) => {
                successful_bans.push(user);
            }
            std::result::Result::Err(BanFailedReason::HelperRestriction(user)) => {
                disallowed_bans.push(user);
            }
            std::result::Result::Err(BanFailedReason::Other(err)) => {
                tracing::error!("{}", err);
                let _ = ctx
                    .say_error(format!(
                        "Something went wrong banning {} ({})",
                        user,
                        user.mention()
                    ))
                    .await;
            }
        }
    }

    if !disallowed_bans.is_empty() {
        let _ = ctx.say_error(
            format!(
                "Failed to ban the following users because of the 3 day account / join age restriction for helpers:\n{}", 
                disallowed_bans.into_iter().map(|x| format!("- {} ({})", x.tag(), x.id)).join("\n")
            )
        ).await;
    }

    if !successful_bans.is_empty() {
        let success_msg = ctx
            .say_success_mod_action(format!(
                "successfully yote\n{}",
                successful_bans
                    .iter()
                    .map(|x| format!("- {} ({})", x.tag(), x.id))
                    .join("\n")
            ))
            .await?
            .message()
            .await?;

        crate::modlog::log_ban(ctx, &success_msg, &successful_bans, &reason).await;
    }

    Ok(())
}

enum BanFailedReason {
    HelperRestriction(User),
    Other(anyhow::Error),
}
impl From<anyhow::Error> for BanFailedReason {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e)
    }
}

async fn handle_single_ban(
    ctx: Ctx<'_>,
    guild: &Guild,
    permission_level: PermissionLevel,
    user: User,
    reason: &str,
    delete_days: u8,
) -> Result<User, BanFailedReason> {
    let ban_allowed = if permission_level == PermissionLevel::Helper {
        let member = guild.member(&ctx.discord(), user.id).await;
        let join_or_create_date = member
            .ok()
            .and_then(|x| x.joined_at)
            .unwrap_or_else(|| user.created_at());
        Utc::now().signed_duration_since(*join_or_create_date) < Duration::days(3)
    } else {
        permission_level == PermissionLevel::Mod
    };

    if !ban_allowed {
        return Err(BanFailedReason::HelperRestriction(user));
    }

    if reason.to_string().contains("ice") {
        let _ = user
            .dm(&ctx.discord(), |m| -> &mut serenity::builder::CreateMessage {
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
            .dm(&ctx.discord(), |m| {
                m.embed(|e| {
                    e.title(format!("You were banned from {}", guild.name));
                    e.field("Reason", reason, false)
                })
            })
            .await;
    }

    let db = ctx.get_db();
    guild
        .ban_with_reason(&ctx.discord(), &user, delete_days, reason)
        .await
        .context("Ban failed")?;

    // Log the ban as a Note in the database
    db.add_ban(
        ctx.author().id,
        user.id,
        reason.to_string(),
        Utc::now(),
        None, // Some(msg.link()), //TODORW
    )
    .await?;

    Ok(user)
}
