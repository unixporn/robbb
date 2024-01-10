use poise::CreateReply;

use super::*;

/// Ping all online moderators. Do not abuse!
#[poise::command(slash_command, guild_only, prefix_command)]
pub async fn modping(
    ctx: Ctx<'_>,
    #[description = "Why are you modpinging?"]
    #[rest]
    reason: String,
) -> Res<()> {
    use poise::serenity_prelude::OnlineStatus::*;
    let config = ctx.get_config();
    let guild = ctx.guild().user_error("not in a guild")?.to_owned();

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

    ctx.send(CreateReply::default().content(format!(
        "{} pinged staff {} for reason {}",
        ctx.author().mention(),
        mods,
        reason
    )))
    .await?;

    Ok(())
}
