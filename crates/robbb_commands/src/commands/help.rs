use poise::serenity_prelude::Message;
use robbb_util::embeds;

use super::*;

/// Show this list
#[poise::command(slash_command, guild_only, track_edits, prefix_command)]
pub async fn help(
    ctx: Ctx<'_>,
    #[description = "The command to get help for."]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Res<()> {
    let commands: Vec<_> = ctx
        .framework()
        .options()
        .commands
        .iter()
        .filter(|x| !x.hide_in_help && (x.slash_action.is_some() || x.prefix_action.is_some()))
        .collect();

    if let Some(desired_command) = command {
        let command = commands
            .iter()
            .find(|c| {
                c.name == desired_command.as_str() || c.aliases.contains(&desired_command.as_str())
            })
            .user_error(&format!("Unknown command `{}`", desired_command))?;
        reply_help_single(ctx, &command).await?;
    } else {
        // Defer, because running these checks currently takes,... longer than it should.
        ctx.defer().await?;
        // find commands that the user has access to
        // TODORW parallelizing this doesn't seem to help at all :thonk:
        let available_commands = commands.into_iter().map(|cmd| async move {
            if let Some(check) = cmd.check {
                match check(ctx.clone()).await {
                    Ok(true) => Some(cmd),
                    Ok(false) => None,
                    Err(e) => {
                        tracing::error!(error = %e, "Error while running check");
                        Some(cmd)
                    }
                }
            } else {
                Some(cmd)
            }
        });
        let available_commands: Vec<Option<&Command<_, _>>> =
            futures::future::join_all(available_commands).await;
        let available_commands = available_commands
            .into_iter()
            .filter_map(|x| x)
            .collect_vec();

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

    Ok(embeds::PaginatedEmbed::create_from_fields(
        fields,
        embeds::make_create_embed(&ctx.discord(), |e| e.title("Help")).await,
    )
    .await
    .reply_to(ctx)
    .await?)
}
