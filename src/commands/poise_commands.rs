use poise::serenity_prelude::ApplicationCommand;

use super::*;

#[poise::command(slash_command, prefix_command, hide_in_help)]
pub async fn delete(
    ctx: Ctx<'_>,
    #[description = "global"]
    #[flag]
    global: bool,
) -> Res<()> {
    if global {
        let global_commands =
            ApplicationCommand::get_global_application_commands(ctx.discord()).await?;
        for command in &global_commands {
            ApplicationCommand::delete_global_application_command(ctx.discord(), command.id)
                .await?;
        }

        ctx.say_success(format!(
            "Deleted global commands: {}",
            global_commands
                .iter()
                .map(|x| x.name.to_string())
                .join(", ")
        ))
        .await?;
    } else {
        if let Some(guild) = ctx.guild() {
            let commands = guild.get_application_commands(ctx.discord()).await?;
            for command in &commands {
                tracing::debug!(deleted_command_name = %command.name, "Deleting application command {}", command.name);
                guild
                    .delete_application_command(ctx.discord(), command.id)
                    .await?;
            }
            ctx.say_success(format!(
                "Deleted application commands: {}",
                commands.iter().map(|x| x.name.to_string()).join(", ")
            ))
            .await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command, prefix_command, hide_in_help)]
pub async fn register(
    ctx: Ctx<'_>,
    #[description = "global"]
    #[flag]
    global: bool,
) -> Res<()> {
    if global {
        poise::builtins::register_application_commands(ctx, true).await?;
    } else {
        if let Some(guild) = ctx.guild() {
            let new_commands = &ctx.framework().options().commands;
            let commands_builder = poise::builtins::create_application_commands(new_commands);
            println!("{:?}", new_commands);

            guild
                .set_application_commands(ctx.discord(), |b| {
                    *b = commands_builder;
                    b
                })
                .await?;

            ctx.say_success(format!(
                "Registered commands: {}",
                new_commands.iter().map(|x| x.name).join(", ")
            ))
            .await?;
        }
    }

    Ok(())
}
