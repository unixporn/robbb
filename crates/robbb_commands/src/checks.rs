use poise::serenity_prelude::{RoleId, User};
use robbb_util::{
    extensions::{ClientContextExt, PoiseContextExt},
    prelude::{Ctx, Res},
};
use serenity::client;

/// Check if the channel allows the use of the given command.
/// This includes specifically checking for /ask in #tech-support
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
    check_role(&ctx.discord(), ctx.author(), config.role_mod).await
}

pub async fn check_is_helper(ctx: Ctx<'_>) -> Res<bool> {
    let config = ctx.get_config();
    check_role(&ctx.discord(), ctx.author(), config.role_helper).await
}

pub async fn check_is_helper_or_mod(ctx: Ctx<'_>) -> Res<bool> {
    let permission_level = get_permission_level(&ctx.discord(), ctx.author()).await?;
    match permission_level {
        PermissionLevel::User => Ok(false),
        PermissionLevel::Helper | PermissionLevel::Mod => Ok(true),
    }
}

pub async fn check_is_not_muted(ctx: Ctx<'_>) -> Res<bool> {
    let config = ctx.get_config();

    check_role(&ctx.discord(), ctx.author(), config.role_mute)
        .await
        .map(|x| !x)
}

#[tracing::instrument(skip_all, fields(user_id = %user.id.0, role_id = %role.0))]
async fn check_role(ctx: &client::Context, user: &User, role: RoleId) -> Res<bool> {
    let config = ctx.get_config().await;
    Ok(user.has_role(ctx, config.guild, role).await?)
}

/// Level of permission a given user has. Ordered such that Mod > Helper > User.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    User,
    Helper,
    Mod,
}

#[tracing::instrument(skip_all)]
pub async fn get_permission_level(ctx: &client::Context, user: &User) -> Res<PermissionLevel> {
    let config = ctx.get_config().await;
    let (mod_check, helper_check) = tokio::join!(
        check_role(ctx, user, config.role_mod),
        check_role(ctx, user, config.role_helper),
    );

    Ok(if mod_check? {
        PermissionLevel::Mod
    } else if helper_check? {
        PermissionLevel::Helper
    } else {
        PermissionLevel::User
    })
}
