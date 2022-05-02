use crate::db::note::NoteType;
use chrono::Utc;
use tracing_futures::Instrument;

use super::*;

#[tracing::instrument(skip_all, fields(blocklist.blocked_word, msg.author = %msg.author.tag(), %msg.id))]
/// If the message contains a blocked word, delete the message, notify the user and #bot-auto-mod.
pub async fn handle_blocklist(ctx: &client::Context, msg: &Message) -> Result<bool> {
    // TODORW
    /*
    let (config, db) = ctx.get_config_and_db().await;

    // remove invisible characters
    let normalized_msg = msg.content.replace(
        &['\u{200b}', '\u{200c}', '\u{200d}', '\u{200e}', '\u{200f}'][..],
        "",
    );

    let blocklist_regex = db.get_combined_blocklist_regex().await?;
    if let Some(word) = blocklist_regex.find(&normalized_msg) {
        // TODORW
        //if checks::get_permission_level(&ctx, &msg).await == PermissionLevel::Mod {
        //tracing::info!("Moderator sent blocked word. Ignoring...");
        //return Ok(false);
        //}

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
        }
        .instrument(tracing::debug_span!("blocklist-dm"));

        let bot_log_future = config
            .log_automod_action(&ctx, |e| {
                e.author(|a| a.name("Message Autodelete"));
                e.title(format!(
                    "{} ({}) - deleted because of `{}`",
                    msg.author.tag(),
                    msg.author.id,
                    word,
                ));
                e.description(format!("{} {}", msg.content, msg.to_context_link()));
            })
            .instrument(tracing::debug_span!("blocklist-automod-entry"));

        let note_future = async {
            let bot_id = ctx.cache.current_user_id();
            let note_content = format!("Message deleted because of word `{}`", word);
            let _ = db
                .add_note(
                    bot_id,
                    msg.author.id,
                    note_content,
                    Utc::now(),
                    NoteType::BlocklistViolation,
                    Some(msg.link()),
                )
                .await;
        }
        .instrument(tracing::debug_span!("blocklist-note"));

        // well, msg.delete does not work for some reason,...
        let delete_future = msg
            .channel_id
            .delete_message(ctx, msg.id)
            .instrument(tracing::debug_span!("blocklist-delete"));

        tokio::join!(dm_future, bot_log_future, note_future, delete_future).3?;

        Ok(true)
    } else {
        Ok(false)
    }
*/
    Ok(false)
}
