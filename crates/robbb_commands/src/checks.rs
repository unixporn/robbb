use anyhow::bail;
use poise::serenity_prelude::RoleId;
use robbb_util::{
    extensions::PoiseContextExt,
    prelude::{Ctx, Res},
};

/// Check if the channel allows the use of the given command.
/// This includes specifically checking for !ask in #tech-support
pub async fn check_channel_allows_commands(ctx: Ctx<'_>) -> Res<bool> {
    let config = ctx.get_config();
    let channel_id = ctx.channel_id();
    if channel_id == config.channel_showcase
        || channel_id == config.channel_feedback
        || (channel_id == config.channel_tech_support && ctx.command().name != "ask")
    {
        Ok(false)
    } else {
        Ok(true)
    }
}

pub async fn check_is_moderator(ctx: Ctx<'_>) -> Res<bool> {
    let config = ctx.get_config();
    check_role(ctx, config.role_mod).await
}

pub async fn check_is_helper(ctx: Ctx<'_>) -> Res<bool> {
    let config = ctx.get_config();
    check_role(ctx, config.role_helper).await
}

pub async fn check_is_helper_or_mod(ctx: Ctx<'_>) -> Res<bool> {
    let config = ctx.get_config();

    let (mod_check, helper_check) = tokio::join!(
        check_role(ctx.clone(), config.role_mod),
        check_role(ctx.clone(), config.role_helper),
    );

    if mod_check.is_ok() || helper_check.is_ok() {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn check_is_not_muted(ctx: Ctx<'_>) -> Res<bool> {
    let config = ctx.get_config();

    check_role(ctx, config.role_mute).await.map(|x| !x)
}

#[tracing::instrument(skip_all, fields(user_id = %ctx.author().id.0, role_id = %role.0))]
async fn check_role(ctx: Ctx<'_>, role: RoleId) -> Res<bool> {
    Ok(match ctx.guild_id() {
        Some(guild_id) => ctx.author().has_role(ctx.discord(), guild_id, role).await?,
        _ => bail!("Not in a guild"),
    })
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PermissionLevel {
    Mod,
    Helper,
    User,
}

#[tracing::instrument(skip_all)]
pub async fn get_permission_level(ctx: Ctx<'_>) -> PermissionLevel {
    let config = ctx.get_config();
    let (mod_check, helper_check) = tokio::join!(
        check_role(ctx.clone(), config.role_mod),
        check_role(ctx.clone(), config.role_helper),
    );

    if mod_check.is_ok() {
        PermissionLevel::Mod
    } else if helper_check.is_ok() {
        PermissionLevel::Helper
    } else {
        PermissionLevel::User
    }
}
