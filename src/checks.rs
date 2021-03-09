use super::Config;
use serenity::client;
use serenity::framework::standard::macros::check;
use serenity::framework::standard::Reason;
use serenity::model::prelude::*;

#[check]
#[name = "Moderator"]
pub async fn moderator_check(ctx: &client::Context, msg: &Message) -> Result<(), Reason> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    check_role(&ctx, msg, config.role_mod).await
}

#[check]
#[name = "Helper"]
pub async fn helper_check(ctx: &client::Context, msg: &Message) -> Result<(), Reason> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    check_role(&ctx, msg, config.role_helper).await
}

#[check]
#[name = "Mute"]
pub async fn mute_check(ctx: &client::Context, msg: &Message) -> Result<(), Reason> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    check_role(&ctx, msg, config.role_mute).await
}

async fn check_role(ctx: &client::Context, msg: &Message, role: RoleId) -> Result<(), Reason> {
    match msg.guild_id {
        Some(guild_id) => match msg.author.has_role(&ctx, guild_id, role).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(Reason::User("Insufficient permissions.".to_string())),
            Err(err) => Err(Reason::UserAndLog {
                user: "Something went wrong while checking for permissions".to_string(),
                log: format!("failed to check role of user {}: {}", msg.author.name, err),
            }),
        },
        _ => Err(Reason::User("Not in a guild.".to_string())),
    }
}
