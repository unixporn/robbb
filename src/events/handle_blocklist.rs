use crate::{
    checks::{self, PermissionLevel},
    db::note::NoteType,
};
use chrono::Utc;

use super::*;

/// If the message contains a blocked word, delete the message, notify the user and #bot-auto-mod.
pub async fn handle_blocklist(ctx: &client::Context, msg: &Message) -> Result<bool> {
    // allow mods to remove from blocklist
    let permission_level = checks::get_permission_level(&ctx, &msg).await;
    if permission_level == PermissionLevel::Mod && msg.content.starts_with("!blocklist") {
        return Ok(false);
    }

    let (config, db) = ctx.get_config_and_db().await;

    // remove invisible characters
    let normalized_msg = msg.content.replace(
        &['\u{200b}', '\u{200c}', '\u{200d}', '\u{200e}', '\u{200f}'][..],
        "",
    );

    let blocklist_regex = db.get_combined_blocklist_regex().await?;
    if let Some(word) = blocklist_regex.find(&normalized_msg) {
        let word = word.as_str();

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
            e.author(|a| a.name("Message Autodelete"));
            e.title(format!(
                "{} - deleted because of `{}`",
                msg.author.tag(),
                word,
            ));
            e.description(format!("{} {}", msg.content, msg.to_context_link()));
        });

        let note_future = async {
            let bot_id = ctx.cache.current_user_id().await;
            let note_content = format!("Message deleted because of word `{}`", word);
            let _ = db
                .add_note(
                    bot_id,
                    msg.author.id,
                    note_content,
                    Utc::now(),
                    NoteType::BlocklistViolation,
                )
                .await;
        };

        // well, msg.delete does not work for some reason,...
        let delete_future = msg.channel_id.delete_message(ctx, msg.id);

        tokio::join!(dm_future, bot_log_future, note_future, delete_future).3?;

        Ok(true)
    } else {
        Ok(false)
    }
}
