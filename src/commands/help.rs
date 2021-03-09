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
    _args: Args,
    _help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    _owners: HashSet<UserId>,
) -> CommandResult {
    let mut commands = Vec::new();
    for group in groups {
        for command in group.options.commands {
            if help_commands::has_all_requirements(&ctx, command.options, msg).await {
                commands.push(command.options)
            }
        }
    }

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
    .await?;
    Ok(())
}
