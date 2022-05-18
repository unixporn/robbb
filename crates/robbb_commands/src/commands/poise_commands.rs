use poise::serenity_prelude::ApplicationCommand;

use super::*;

/// Unregister all slash commands (be careful)
///
/// Slash commands have to be explicitly registered with discord.
/// To remove them again, you can use this command.
/// Please only run this when absolutely necessary, as setting up the permissions for the commands again is pain.
#[poise::command(
    slash_command,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    hide_in_help
)]
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
            tracing::debug!(deleted_command_name = %command.name, "Deleting global application command {}", command.name);
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
    } else if let Some(guild) = ctx.guild() {
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
    Ok(())
}

/// Register all slash commands with discords API
///
/// Slash commands have to be explicitly registered with discord, which you can do via this command.
#[poise::command(
    slash_command,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    hide_in_help
)]
pub async fn register(
    ctx: Ctx<'_>,
    #[description = "global"]
    #[flag]
    global: bool,
) -> Res<()> {
    let new_commands = &ctx.framework().options().commands;
    let commands_builder = poise::builtins::create_application_commands(new_commands);
    println!("{:?}", new_commands);
    if global {
        ApplicationCommand::set_global_application_commands(ctx.discord(), |b| {
            *b = commands_builder;
            b
        })
        .await?;
    } else if let Some(guild) = ctx.guild() {
        guild
            .set_application_commands(ctx.discord(), |b| {
                *b = commands_builder;
                b
            })
            .await?;
    }

    ctx.say_success(format!(
        "Registered commands: {}",
        new_commands.iter().map(|x| x.name).join(", ")
    ))
    .await?;

    Ok(())
}
