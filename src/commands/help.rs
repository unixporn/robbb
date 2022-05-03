use poise::serenity_prelude::Message;

use crate::embeds::{self, PaginatedEmbed};

use super::*;

#[poise::command(slash_command, guild_only, track_edits, prefix_command)]
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
        .filter(|x| x.slash_action.is_some() || x.prefix_action.is_some());

    if let Some(desired_command) = command {
        let command = commands
            .find(|c| {
                c.name == desired_command.as_str() || c.aliases.contains(&desired_command.as_str())
            })
            .user_error(&format!("Unknown command `{}`", desired_command))?;
        reply_help_single(ctx, &command).await?;
    } else {
        // find commands that the user has access to
        let mut available_commands = Vec::new();
        for cmd in commands {
            let check_success = if let Some(check) = cmd.check {
                check(ctx.clone()).await?
            } else {
                true
            };

            if check_success {
                available_commands.push(cmd);
            }

            // TODORW

            //if help_commands::has_all_requirements(&ctx, cmd.options, msg)
            //&& passes_all_checks(cmd.check, &ctx, &msg, &mut args, cmd.options).await
            //{
            //commands.push(cmd.options)
            //}
        }
        reply_help_full(ctx, &available_commands).await?;
    }

    Ok(())
}

async fn reply_help_single(ctx: Ctx<'_>, command: &Command<UserData, Error>) -> Res<Message> {
    let handle = ctx
        .send_embed(move |e| {
            e.title(format!("Help for {}", command.name));
            if let Some(desc) = command.multiline_help {
                e.description(desc());
            } else if let Some(help) = command.inline_help {
                e.description(help);
            }

            if !command.subcommands.is_empty() {
                let subcommands_text = command
                    .subcommands
                    .iter()
                    .map(|subcommand| {
                        if let Some(usage) = subcommand.inline_help {
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
    Ok(handle.message().await?)
}

async fn reply_help_full(ctx: Ctx<'_>, commands: &[&Command<UserData, Error>]) -> Res<Message> {
    let fields = commands.iter().map(|command| {
        let name = format!("**/{}**", command.name);
        let description = command.inline_help.unwrap_or("No description").to_string();
        (name, description)
    });

    Ok(PaginatedEmbed::create_from_fields(
        fields,
        embeds::make_create_embed(&ctx.discord(), |e| e.title("Help")).await,
    )
    .await
    .reply_to(ctx)
    .await?)
}
