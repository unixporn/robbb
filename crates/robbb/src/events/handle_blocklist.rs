use chrono::Utc;
use poise::serenity_prelude::message_component::ActionRowComponent;
use robbb_commands::checks::{self, PermissionLevel};
use robbb_db::mod_action::ModActionKind;
use tracing_futures::Instrument;

use super::*;

const INVISIBLE_CHARS: &[char] = &['\u{200b}', '\u{200c}', '\u{200d}', '\u{200e}', '\u{200f}'];

/// If the message contains a blocked word, delete the message, notify the user and #bot-auto-mod.
/// Returns true if the message had a blocked word, in which case handling the message_create event should be stopped early.
#[tracing::instrument(skip_all, fields(blocklist.blocked_word, msg.author = %msg.author.tag(), %msg.id))]
pub async fn handle_blocklist(ctx: &client::Context, msg: &Message) -> Result<bool> {
    let (config, db) = ctx.get_config_and_db().await;

    // remove invisible characters
    let normalized_msg = msg.content.replace(INVISIBLE_CHARS, "");
    let blocklist_regex = db.get_combined_blocklist_regex().await?;
    if let Some(word) = blocklist_regex.find(&normalized_msg) {
        if checks::get_permission_level(&ctx, &msg.author).await? == PermissionLevel::Mod {
            return Ok(false);
        }

        let word = word.as_str();

        tracing::info!(blocklist.word = %word, "Found blocked word '{}'", word);
        tracing::Span::current().record("blocklist.blocked_word", &word);

        let dm_future = async {
            let _ = msg
                .author
                .dm(&ctx, |m| {
                    m.embed(|e| {
                        e.description(&msg.content).title(format!(
                            "Your message has been deleted for containing a blocked word: `{}`",
                            word
                        ))
                    })
                })
                .await;
        };

        let bot_log_future = config.log_automod_action(&ctx, |e| {
            e.author_user(&msg.author);
            e.title("Message Autodelete");
            e.field("Deleted because of", word, false);
            e.description(format!("{}\n{}", msg.content, msg.to_context_link()));
        });

        let note_future = async {
            let bot_id = ctx.cache.current_user_id();
            let note_content = format!("Message deleted because of word `{}`", word);
            let _ = db
                .add_mod_action(
                    bot_id,
                    msg.author.id,
                    note_content,
                    Utc::now(),
                    Some(msg.link()),
                    ModActionKind::BlocklistViolation,
                )
                .await;
        };

        // well, msg.delete does not work for some reason,...
        let delete_future = msg.channel_id.delete_message(ctx, msg.id);

        tokio::join!(
            dm_future.instrument(tracing::debug_span!("blocklist-dm")),
            bot_log_future.instrument(tracing::debug_span!("blocklist-automod-entry")),
            note_future.instrument(tracing::debug_span!("blocklist-note")),
            delete_future.instrument(tracing::debug_span!("blocklist-delete"))
        )
        .3?;

        Ok(true)
    } else {
        Ok(false)
    }
}

// TODORW this doesn't yet really work for modals, apparently...
/// Returns true if the interaction had a blocked word, in which case handling the interaction event should be stopped early.
#[tracing::instrument(skip_all, fields(blocklist.blocked_word, interaction.user))]
pub async fn handle_blocklist_in_interaction(
    ctx: &client::Context,
    interaction: &Interaction,
) -> Result<bool> {
    let values = match collect_interaction_values(interaction) {
        Some(x) if !x.values.is_empty() => x,
        _ => return Ok(false),
    };

    let (config, db) = ctx.get_config_and_db().await;
    let blocklist_regex = db.get_combined_blocklist_regex().await?;
    for value in &values.values {
        let normalized = value.replace(INVISIBLE_CHARS, "");
        if let Some(word) = blocklist_regex.find(&normalized) {
            if checks::get_permission_level(&ctx, &values.user).await? == PermissionLevel::Mod {
                return Ok(false);
            }
            let word = word.as_str();

            tracing::info!(blocklist.word = %word, "Found blocked word in interaction '{}'", word);
            tracing::Span::current().record("blocklist.blocked_word", &word);
            tracing::Span::current().record("interaction.user", &values.user.tag().as_str());

            let bot_log_future = config.log_automod_action(&ctx, |e| {
                e.author_user(&values.user);
                e.title("Interaction aborted because of blocked word");
                e.field("Aborted because of", word, false);
                e.field("Interaction", values.title, false);
            });

            let note_future = async {
                let bot_id = ctx.cache.current_user_id();
                let note_content = format!(
                    "Interaction `{}` interrupted because of word `{}`",
                    values.title, word
                );
                let _ = db
                    .add_mod_action(
                        bot_id,
                        values.user.id,
                        note_content,
                        Utc::now(),
                        None,
                        ModActionKind::BlocklistViolation,
                    )
                    .await;
            };

            let reply_future = async {
                match interaction {
                    Interaction::ApplicationCommand(x) => {
                        let _ = x
                            .create_interaction_response(&ctx, |ir| {
                                ir.interaction_response_data(|m| m.content("Bruh"))
                            })
                            .await;
                    }
                    Interaction::ModalSubmit(x) => {
                        let _ = x
                            .create_interaction_response(&ctx, |ir| {
                                ir.interaction_response_data(|m| m.content("Bruh"))
                            })
                            .await;
                    }
                    _ => {}
                }
            };

            tokio::join!(
                bot_log_future.instrument(tracing::debug_span!("blocklist-automod-entry")),
                note_future.instrument(tracing::debug_span!("blocklist-note")),
                reply_future.instrument(tracing::debug_span!("blocklist-interaction-response"))
            );
            return Ok(true);
        }
    }

    Ok(false)
}

struct InteractionValues<'a> {
    values: Vec<&'a str>,
    user: &'a User,
    title: &'a str,
}

fn collect_interaction_values(interaction: &Interaction) -> Option<InteractionValues> {
    match interaction {
        Interaction::Ping(_) | Interaction::MessageComponent(_) | Interaction::Autocomplete(_) => {
            None
        }
        Interaction::ApplicationCommand(command_interaction) => {
            let values = command_interaction
                .data
                .options
                .iter()
                .flat_map(|x| get_options_from_option(x).into_iter())
                .collect();

            Some(InteractionValues {
                values,
                user: &command_interaction.user,
                title: &command_interaction.data.name,
            })
        }
        Interaction::ModalSubmit(modal_interaction) => Some(InteractionValues {
            values: modal_interaction
                .data
                .components
                .iter()
                .flat_map(|x| x.components.iter())
                .filter_map(|x| match x {
                    ActionRowComponent::InputText(text) => Some(text.value.as_str()),
                    _ => None,
                })
                .collect(),
            user: &modal_interaction.user,
            title: "Modal",
        }),
    }
}

/// Recursively traverse an `ApplicationCommandInteractionDataOption`
/// to get all the `ApplicationCommandInteractionDataOptionValue`s of it and it's subcommands / groups
fn get_options_from_option(
    option: &application_command::ApplicationCommandInteractionDataOption,
) -> Vec<&str> {
    let mut values = Vec::new();
    if let Some(value) = option.value.as_ref().and_then(|x| x.as_str()) {
        values.push(value);
    }
    for option in &option.options {
        values.append(&mut get_options_from_option(option));
    }
    values
}
