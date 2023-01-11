use poise::serenity_prelude::Message;
use robbb_util::embeds;

use crate::checks;

use super::*;

/// Show this list
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn help(
    ctx: Ctx<'_>,
    #[description = "The command to get help for."]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Res<()> {
    let mut commands = ctx
        .framework()
        .options()
        .commands
        .iter()
        .filter(|x| !x.hide_in_help && (x.slash_action.is_some() || x.prefix_action.is_some()));

    if let Some(desired_command) = command {
        let command = commands
            .find(|c| {
                c.name == desired_command.as_str() || c.aliases.contains(&desired_command.as_str())
            })
            .user_error(&format!("Unknown command `{}`", desired_command))?;
        reply_help_single(ctx, command).await?;
    } else {
        let permission_level =
            checks::get_permission_level(ctx.serenity_context(), ctx.author()).await?;
        let available_commands: Vec<_> = commands
            .filter(|command| {
                command
                    .custom_data
                    .downcast_ref::<CmdMeta>()
                    .map_or(true, |meta| permission_level >= meta.perms)
            })
            .collect();

        reply_help_full(ctx, &available_commands).await?;
    }
    Ok(())
}

async fn reply_help_single(ctx: Ctx<'_>, command: &Command<UserData, Error>) -> Res<Message> {
    let handle = ctx
        .send_embed_full(true, move |e| {
            e.title(format!("Help for {}", command.name));
            if let Some(desc) = command.help_text {
                e.description(desc());
            } else if let Some(help) = &command.description {
                e.description(help);
            }

            if !command.subcommands.is_empty() {
                let subcommands_text = command
                    .subcommands
                    .iter()
                    .map(|subcommand| {
                        if let Some(usage) = &subcommand.description {
                            format!("**/{} {}** - ``{} ``", command.name, subcommand.name, usage)
                        } else {
                            format!("**/{} {}**", command.name, subcommand.name)
                        }
                    })
                    .join("\n");

                e.field("Subcommands", subcommands_text, false);
            }
        })
        .await?;
    Ok(handle.message().await?.into_owned())
}

async fn reply_help_full(ctx: Ctx<'_>, commands: &[&Command<UserData, Error>]) -> Res<Message> {
    let fields = commands.iter().map(|command| {
        let name = if command.slash_action.is_some() {
            format!("**/{}**", command.name)
        } else {
            format!("**!{}**", command.name)
        };
        let description = command.description.as_deref().unwrap_or("No description").to_string();
        (name, description)
    });

    embeds::PaginatedEmbed::create_from_fields(
        "Help".to_string(),
        fields,
        embeds::make_create_embed(ctx.serenity_context(), |e| e).await,
    )
    .await
    .reply_to(ctx, true)
    .await
}
