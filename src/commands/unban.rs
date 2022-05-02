use super::*;
/// Unban a user.
#[command]
#[only_in(guilds)]
#[usage("unban <user>")]
pub async fn unban(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let user_id = args
        .single::<UserId>()
        .invalid_usage(&UNBAN_COMMAND_OPTIONS)?;
    let user = user_id.to_user(&ctx).await?;

    guild.unban(&ctx, user_id).await?;

    msg.reply_success(&ctx, format!("Succesfully deyote {}", user_id.mention()))
        .await?;

    modlog::log_unban(ctx, msg, user).await;

    Ok(())
}
