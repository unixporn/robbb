use super::*;

/// Ping all online moderators. Do not abuse!
#[command]
#[only_in(guilds)]
#[usage("modping <reason>")]
pub async fn modping(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    use OnlineStatus::*;

    let config = ctx.get_config().await;
    let reason = args.remains().invalid_usage(&MODPING_COMMAND_OPTIONS)?;

    let guild = msg.guild(&ctx).context("Failed to fetch guild")?;

    let mods_and_helpers = guild
        .members_with_status(Online)
        .into_iter()
        .chain(guild.members_with_status(Idle).into_iter())
        .chain(guild.members_with_status(DoNotDisturb).into_iter())
        .filter(|member| {
            member.roles.contains(&config.role_mod) || member.roles.contains(&config.role_helper)
        })
        .collect_vec();

    let mods = if mods_and_helpers.len() < 2 {
        // ping moderator role if no helpers nor mods are available
        config.role_mod.mention().to_string()
    } else {
        mods_and_helpers.iter().map(|m| m.mention()).join(", ")
    };

    msg.channel_id
        .send_message(&ctx, |m| {
            m.content(format!(
                "{} pinged staff {} for reason {}",
                msg.author.mention(),
                mods,
                reason,
            ))
        })
        .await?;

    Ok(())
}
