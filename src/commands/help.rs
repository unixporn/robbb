use serenity::framework::standard::Command;

use super::*;

#[help]
#[individual_command_tip = "If you want more information about a specific command, just pass the command as argument."]
#[command_not_found_text = "Could not find: `{}`."]
#[max_levenshtein_distance(3)]
#[indention_prefix = "+"]
#[lacking_permissions = "Hide"]
#[lacking_role = "Nothing"]
#[wrong_channel = "Strike"]
async fn my_help(
    ctx: &client::Context,
    msg: &Message,
    mut args: Args,
    _help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    _owners: HashSet<UserId>,
) -> CommandResult {
    let desired_command = args.single::<String>().ok();
    if let Some(desired_command) = desired_command {
        let command = groups
            .into_iter()
            .find_map(|g| {
                g.options
                    .commands
                    .into_iter()
                    .find(|c| c.options.names.contains(&desired_command.as_str()))
            })
            .user_error(&format!("Unknown command `{}`", desired_command))?;
        reply_help_single(&ctx, &msg, &command).await?;
    } else {
        let mut commands = Vec::new();
        for group in groups {
            'command: for command in group.options.commands {
                for check in group.options.checks {
                    if (check.function)(&ctx, &msg, &mut args, &command.options)
                        .await
                        .is_err()
                    {
                        continue 'command;
                    }
                }
                if help_commands::has_all_requirements(&ctx, command.options, msg).await {
                    commands.push(command.options)
                }
            }
        }
        reply_help_full(&ctx, &msg, &commands).await?;
    }
    Ok(())
}

async fn reply_help_single(
    ctx: &client::Context,
    msg: &Message,
    command: &Command,
) -> Result<Message> {
    msg.reply_embed(&ctx, move |e| {
        e.title(format!(
            "Help for {}",
            command.options.names.first().unwrap()
        ));
        command.options.desc.map(|d| e.description(d));
        command.options.usage.map(|u| e.field("Usage", u, false));
    })
    .await
}

async fn reply_help_full(
    ctx: &client::Context,
    msg: &Message,
    commands: &[&CommandOptions],
) -> Result<Message> {
    msg.reply_embed(&ctx, move |e| {
        e.title("Help");
        for command in commands {
            let command_name = command.names.first().expect("Command had no name");
            let name = match command.usage {
                Some(usage) => format!("**{}** - {}", command_name, usage),
                None => format!("**{}**", command_name),
            };
            let description = command.desc.unwrap_or("No description").to_string();
            let description = if !command.examples.is_empty() {
                format!("{}\n{}", description, command.examples.join("\n"))
            } else {
                description
            };
            e.field(name, description, false);
        }
    })
    .await
}
