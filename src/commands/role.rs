use super::*;

/// Set your role. Use without arguments to see available roles.
#[command]
#[only_in(guilds)]
#[usage("role [role-name]")]
#[aliases("roles")]
pub async fn role(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let config = ctx.get_config().await;

    if let Some(chosen_role_name) = args.remains() {
        let guild = msg.guild(&ctx).context("Failed to load guild")?;
        let chosen_role = config
            .roles_color
            .iter()
            .filter_map(|r| guild.roles.get(r))
            .find(|r| {
                r.name == chosen_role_name || Some(r.id) == chosen_role_name.parse::<RoleId>().ok()
            })
            .user_error("Unknown color role")?;

        let mut member = guild.member(&ctx, msg.author.id).await?;
        let old_roles = member.roles.clone();

        member.remove_roles(&ctx, &config.roles_color).await?;

        if !old_roles.contains(&chosen_role.id) {
            member.add_role(&ctx, chosen_role.id).await?;
            msg.reply_success(
                &ctx,
                format!("Success! You're now {}", chosen_role.id.mention()),
            )
            .await?;
        } else {
            msg.reply_success(&ctx, "Success! Removed your role!")
                .await?;
        }
    } else {
        msg.reply_embed(&ctx, |e| {
            e.title("Available roles");
            e.description(config.roles_color.iter().map(|r| r.mention()).join("\n"));
            e.footer(|f| f.text(format!("Usage: {}", &ROLE_COMMAND_OPTIONS.usage.unwrap())));
        })
        .await?;
    }

    Ok(())
}
