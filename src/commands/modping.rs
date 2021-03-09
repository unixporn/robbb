use super::*;

/// Ping all online moderators. Do not abuse!
#[command]
#[usage("modping <reason>")]
pub async fn modping(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    let reason = args.remains().invalid_usage(&MODPING_COMMAND_OPTIONS)?;

    let guild = msg.guild(&ctx).await.context("Failed to fetch guild")?;
    let mods = guild
        .members
        .values()
        .filter(|member| member.roles.contains(&config.role_mod));

    msg.channel_id
        .send_message(&ctx, |m| {
            m.content(format!(
                "{} pinged moderators {} for reason {}",
                msg.author.mention(),
                mods.map(|m| m.mention()).join(", "),
                reason,
            ))
        })
        .await?;

    Ok(())
}
