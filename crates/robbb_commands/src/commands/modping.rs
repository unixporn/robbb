use poise::CreateReply;

use super::*;

/// Ping all online moderators. Do not abuse!
#[poise::command(slash_command, guild_only)]
pub async fn modping(
    ctx: Ctx<'_>,
    #[description = "Why are you modpinging?"]
    #[rest]
    reason: String,
) -> Res<()> {
    use poise::serenity_prelude::OnlineStatus::*;
    let config = ctx.get_config();
    let guild = ctx.guild().user_error("not in a guild")?.to_owned();

    let online_staff = guild
        .members_with_status(Online)
        .chain(guild.members_with_status(Idle))
        .chain(guild.members_with_status(DoNotDisturb))
        .filter(|member| {
            member.roles.contains(&config.role_mod) || member.roles.contains(&config.role_helper)
        })
        .collect_vec();

    let contains_moderators =
        online_staff.iter().any(|member| member.roles.contains(&config.role_mod));

    let staff_to_ping = if contains_moderators {
        online_staff.iter().map(|m| m.mention()).join(", ")
    } else {
        config.role_mod.mention().to_string()
            + &online_staff.iter().map(|m| m.mention()).join(", ")
    };

    ctx.send(CreateReply::default().content(format!(
        "{} pinged staff {staff_to_ping} for reason {reason}",
        ctx.author().mention()
    )))
    .await?;

    Ok(())
}
