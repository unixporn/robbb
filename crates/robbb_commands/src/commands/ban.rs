use anyhow::Context;
use chrono::{Duration, Utc};
use poise::serenity_prelude::{Message, User};
use robbb_util::embeds;
use serenity::builder::{CreateEmbed, CreateMessage, EditMessage};

use crate::checks::{self, PermissionLevel};

use super::*;

#[derive(poise::Modal)]
#[name = "Ban"]
struct BanModal {
    #[paragraph]
    reason: String,
}

#[poise::command(
    guild_only,
    context_menu_command = "Ban",
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn menu_ban(app_ctx: AppCtx<'_>, user: User) -> Res<()> {
    let response: Option<BanModal> = poise::execute_modal(app_ctx, None, None).await?;
    if let Some(response) = response {
        do_ban(app_ctx.into(), vec![user], response.reason, 0).await?;
    } else {
        Ctx::Application(app_ctx).say_error("Cancelled").await?;
    }
    Ok(())
}

/// Ban a user from the server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Helper }"
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
    rename = "banmany",
    slash_command,
    prefix_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Helper }"
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
            .to_user(&ctx.serenity_context())
            .await?;
        users.push(user_id);
    }

    do_ban(ctx, users, reason, if delete_messages { 1 } else { 0 }).await?;
    Ok(())
}

async fn do_ban(ctx: Ctx<'_>, users: Vec<User>, reason: String, delete_days: u8) -> Res<()> {
    let guild = ctx.guild().context("Failed to load guild")?.to_owned();

    let mut disallowed_bans = Vec::new();
    let mut successful_bans = Vec::new();

    let permission_level =
        checks::get_permission_level(ctx.serenity_context(), ctx.author()).await?;

    let mut main_response = ctx
        .say_success_mod_action(format!("Banning {}...", users.iter().map(|x| x.tag()).join(", ")))
        .await?
        .message()
        .await?
        .into_owned();

    for user in users {
        match handle_single_ban(
            ctx,
            &guild,
            permission_level,
            user.clone(),
            &reason,
            delete_days,
            &main_response,
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
                        user.tag(),
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
        let embed = embeds::make_success_mod_action_embed(
            ctx.serenity_context(),
            &format!(
                "successfully yote\n{}",
                successful_bans
                    .iter()
                    .map(|x| format!("- {} ({}).\nReason: {}", x.tag(), x.id, reason))
                    .join("\n")
            ),
        )
        .await;

        main_response.edit(&ctx.serenity_context(), EditMessage::default().embed(embed)).await?;

        crate::modlog::log_ban(ctx, &main_response, &successful_bans, &reason).await;
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
    ctx_message: &Message,
) -> Result<User, BanFailedReason> {
    let ban_allowed = if permission_level == PermissionLevel::Helper {
        let member = guild.member(&ctx.serenity_context(), user.id).await;
        let join_or_create_date =
            member.ok().and_then(|x| x.joined_at).unwrap_or_else(|| user.created_at());
        Utc::now().signed_duration_since(*join_or_create_date) < Duration::days(3)
    } else {
        permission_level == PermissionLevel::Mod
    };

    if !ban_allowed {
        return Err(BanFailedReason::HelperRestriction(user));
    }

    let _ = user
        .dm(
            &ctx.serenity_context(),
            CreateEmbed::default()
                .title(format!("You were banned from {}", guild.name))
                .field("Reason", reason, false)
                .into_create_message(),
        )
        .await;

    let db = ctx.get_db();
    guild
        .ban_with_reason(&ctx.serenity_context(), &user, delete_days, reason)
        .await
        .context("Ban failed")?;

    // Log the ban as a Note in the database
    db.add_mod_action(
        ctx.author().id,
        user.id,
        reason.to_string(),
        Utc::now(),
        ctx_message.link(),
        robbb_db::mod_action::ModActionKind::Ban,
    )
    .await?;

    Ok(user)
}
