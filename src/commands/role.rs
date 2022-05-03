use poise::serenity_prelude::RoleId;

use super::*;

async fn autocomplete_color_role(ctx: Ctx<'_>, partial: String) -> Vec<String> {
    let config = ctx.get_config();
    let guild = ctx.guild().unwrap();
    config
        .roles_color
        .iter()
        .filter_map(|x| guild.roles.get(&x))
        .map(|x| x.name.to_string())
        .collect()
}

/// Set your role.
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Miscellaneous",
    track_edits
)]
pub async fn role(
    ctx: Ctx<'_>,
    #[description = "The color role you want"]
    #[autocomplete = "autocomplete_color_role"]
    role: Option<String>,
) -> Res<()> {
    let config = ctx.get_config();

    if let Some(chosen_role_name) = role {
        let guild = ctx.guild().expect("guild_only");
        let chosen_role = config
            .roles_color
            .iter()
            .filter_map(|r| guild.roles.get(r))
            .find(|r| {
                r.name == chosen_role_name || Some(r.id) == chosen_role_name.parse::<RoleId>().ok()
            })
            .user_error("Unknown color role")?;

        let mut member = ctx.author_member().await.user_error("Not a member")?;
        let old_roles = member.roles.clone();

        member
            .remove_roles(&ctx.discord(), &config.roles_color)
            .await?;

        if !old_roles.contains(&chosen_role.id) {
            member.add_role(&ctx.discord(), chosen_role.id).await?;
            ctx.say_success(format!("Success! You're now {}", chosen_role.id.mention()))
                .await?;
        } else {
            ctx.say_success("Success! Removed your role!").await?;
        }
    } else {
        ctx.send_embed(|e| {
            e.title("Available roles");
            e.description(config.roles_color.iter().map(|r| r.mention()).join("\n"));
        })
        .await?;
    }

    Ok(())
}
