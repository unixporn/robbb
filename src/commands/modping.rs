use super::*;

/// Ping all online moderators. Do not abuse!
#[command]
#[usage("modping <reason>")]
pub async fn modping(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    use OnlineStatus::DoNotDisturb;
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    let reason = args.remains().invalid_usage(&MODPING_COMMAND_OPTIONS)?;

    let guild = msg.guild(&ctx).await.context("Failed to fetch guild")?;

    let mods = guild
        .members_with_status(OnlineStatus::Online)
        .into_iter()
        .chain(guild.members_with_status(OnlineStatus::Idle).into_iter())
        .chain(guild.members_with_status(DoNotDisturb).into_iter())
        .filter(|member| member.roles.contains(&config.role_mod))
        .collect_vec();

    let mods = if mods.len() < 2 {
        config.role_mod.mention().to_string()
    } else {
        mods.iter().map(|m| m.mention()).join(", ")
    };

    msg.channel_id
        .send_message(&ctx, |m| {
            m.content(format!(
                "{} pinged moderators {} for reason {}",
                msg.author.mention(),
                mods,
                reason,
            ))
        })
        .await?;

    Ok(())
}
