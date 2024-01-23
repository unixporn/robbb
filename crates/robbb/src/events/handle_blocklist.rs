use chrono::Utc;
use robbb_commands::{
    checks::{self, PermissionLevel},
    commands::blocklist::SHOULD_NEVER_TRIGGER_BLOCKLIST,
};
use robbb_db::mod_action::ModActionKind;
use robbb_util::util::{generate_message_link, time_to_discord_snowflake};
use serenity::{
    all::{CommandInteraction, ResolvedValue},
    builder::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage},
};
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
    if SHOULD_NEVER_TRIGGER_BLOCKLIST.iter().any(|x| blocklist_regex.is_match(x)) {
        tracing::error!("Blocklist regex matches one of the sanity check patterns. Make sure none of the blocklist entries match the empty string.");
        return Ok(false);
    }

    if let Some(word) = blocklist_regex.find(&normalized_msg) {
        if checks::get_permission_level(&ctx, &msg.author).await? == PermissionLevel::Mod {
            return Ok(false);
        }

        let word = word.as_str();

        tracing::info!(blocklist.word = %word, "Found blocked word '{}'", word);
        tracing::Span::current().record("blocklist.blocked_word", word);

        let dm_embed = CreateEmbed::default()
            .description(&msg.content)
            .title(
                format!("Your message has been deleted for containing a blocked word: `{word}`",),
            )
            .into_create_message();
        let dm_future = async {
            let _ = msg.author.dm(&ctx, dm_embed).await;
        };

        let bot_log_future = config.log_automod_action(&ctx, |e| {
            e.author_user(&msg.author)
                .title("Message Autodelete")
                .field("Deleted because of", word, false)
                .description(format!("{}\n{}", msg.content, msg.to_context_link()))
        });

        let note_future = async {
            let bot_id = ctx.cache.current_user().id;
            let note_content = format!("Message deleted because of word `{word}`");
            let _ = db
                .add_mod_action(
                    bot_id,
                    msg.author.id,
                    note_content,
                    Utc::now(),
                    msg.link(),
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

/// Returns true if the interaction had a blocked word, in which case handling the interaction event should be stopped early.
#[tracing::instrument(skip_all, fields(blocklist.blocked_word, interaction.user))]
pub async fn handle_blocklist_in_command_interaction(
    ctx: &client::Context,
    interaction: &CommandInteraction,
) -> Result<bool> {
    let values = collect_interaction_values(interaction);

    let db = ctx.get_db().await;
    let blocklist_regex = db.get_combined_blocklist_regex().await?;
    for value in &values.values {
        let normalized = value.replace(INVISIBLE_CHARS, "");
        if checks::get_permission_level(&ctx, &values.user).await? == PermissionLevel::Mod {
            return Ok(false);
        }
        if let Some(word) = blocklist_regex.find(&normalized) {
            handle_blocked_word_in_interaction(ctx, interaction, word.as_str(), values).await;
            return Ok(true);
        }
    }

    Ok(false)
}

async fn handle_blocked_word_in_interaction(
    ctx: &client::Context,
    interaction: &CommandInteraction,
    word: &str,
    values: InteractionValues<'_>,
) {
    let db = ctx.get_db().await;
    tracing::info!(blocklist.word = %word, "Found blocked word in interaction '{word}'");
    tracing::Span::current().record("blocklist.blocked_word", word);
    tracing::Span::current().record("interaction.user", values.user.tag().as_str());

    let bot_log_future = ctx.log_automod_action(|e| {
        e.author_user(&values.user)
            .title("Interaction aborted because of blocked word")
            .field("Aborted because of", word, false)
            .field("Interaction", values.title, false)
    });

    let note_future = async {
        let bot_id = ctx.cache.current_user().id;
        let note_content =
            format!("Interaction `{}` interrupted because of word `{word}`", values.title);
        let context_link = generate_message_link(
            values.guild_id,
            values.channel_id,
            time_to_discord_snowflake(Utc::now()),
        );
        let _ = db
            .add_mod_action(
                bot_id,
                values.user.id,
                note_content,
                Utc::now(),
                context_link,
                ModActionKind::BlocklistViolation,
            )
            .await;
    };

    let reply_future = async {
        let interaction_response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::default().content("Bruh"),
        );
        log_error!(interaction.create_response(&ctx, interaction_response).await);
    };

    tokio::join!(
        bot_log_future.instrument(tracing::debug_span!("blocklist-automod-entry")),
        note_future.instrument(tracing::debug_span!("blocklist-note")),
        reply_future.instrument(tracing::debug_span!("blocklist-interaction-response"))
    );
}

#[derive(Debug)]
struct InteractionValues<'a> {
    values: Vec<&'a str>,
    user: &'a User,
    guild_id: Option<GuildId>,
    channel_id: ChannelId,
    title: &'a str,
}

fn collect_interaction_values(interaction: &CommandInteraction) -> InteractionValues {
    fn values_from_resolved_value<'a>(value: &ResolvedValue<'a>) -> Vec<&'a str> {
        match value {
            ResolvedValue::String(s) => vec![s],
            ResolvedValue::SubCommand(sub) => {
                sub.iter().flat_map(|x| values_from_resolved_value(&x.value).into_iter()).collect()
            }
            ResolvedValue::SubCommandGroup(sub) => {
                sub.iter().flat_map(|x| values_from_resolved_value(&x.value).into_iter()).collect()
            }
            _ => vec![],
        }
    }

    let values = interaction
        .data
        .options()
        .iter()
        .flat_map(|x| values_from_resolved_value(&x.value))
        .collect();

    InteractionValues {
        values,
        channel_id: interaction.channel_id,
        guild_id: interaction.guild_id,
        user: &interaction.user,
        title: &interaction.data.name,
    }
}
