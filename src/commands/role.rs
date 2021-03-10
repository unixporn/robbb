use super::*;

/// Set your role. Use without arguments to see available roles.
#[command]
#[usage("role [role-name]")]
pub async fn role(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let config = data.get::<Config>().unwrap().clone();

    if args.is_empty() {
        msg.reply_embed(&ctx, |e| {
            e.title("Available roles");
            e.description(config.roles_color.iter().map(|r| r.mention()).join("\n"));
            e.footer(|f| f.text(format!("Usage: {}", &ROLE_COMMAND_OPTIONS.usage.unwrap())));
        })
        .await?;
    } else {
        let chosen_role_name = args
            .single::<String>()
            .invalid_usage(&ROLE_COMMAND_OPTIONS)?;

        let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
        let chosen_role = config
            .roles_color
            .iter()
            .filter_map(|r| guild.roles.get(r))
            .find(|r| {
                r.name == chosen_role_name || Some(r.id) == chosen_role_name.parse::<RoleId>().ok()
            })
            .user_error("Unknown color role")?;

        let mut member = guild.member(&ctx, msg.author.id).await?;
        member.remove_roles(&ctx, &config.roles_color).await?;
        member.add_role(&ctx, chosen_role.id).await?;
        msg.reply_success(
            &ctx,
            format!("Success! You're now {}", chosen_role.id.mention()),
        )
        .await?;
    }

    Ok(())
}
