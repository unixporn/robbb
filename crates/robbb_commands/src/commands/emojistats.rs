use super::*;

use std::collections::hash_map::HashMap;

use anyhow::Context;
use poise::serenity_prelude::{Emoji, EmojiId};

use robbb_db::emoji_logging::{EmojiStats, Ordering};

/// Get statistics about the usage of emotes
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn emojistats(
    ctx: Ctx<'_>,
    #[description = "Reverse order of popularity"]
    #[flag]
    #[rename = "ascending"]
    sort_ascending: bool,
    #[description = "The emote you want statistics on"] emote: Option<String>,
) -> Res<()> {
    let db = ctx.get_db();
    let ordering = match sort_ascending {
        true => Ordering::Ascending,
        false => Ordering::Descending,
    };

    let guild_emojis = ctx.get_guild_emojis().context("could not get guild emojis")?;

    match emote {
        Some(emote_name) => {
            let found_emoji = match util::find_emojis(&emote_name).first() {
                Some(searched_emoji) => db.get_emoji_usage_by_id(searched_emoji).await,
                None => db.get_emoji_usage_by_name(&emote_name).await,
            };
            let emoji_data = found_emoji.user_error("Could not find that emote")?;

            let emoji = guild_emojis
                .get(&emoji_data.emoji.id)
                .user_error("Could not find emoji in guild")?;
            ctx.send_embed(|e| {
                e.title(format!("Emoji usage for *{}*", emoji.name));
                e.thumbnail(emoji.url());
                e.description(format!(
                    "**Reactions:** {} \n**In Text:** {} \n**Total:** {}",
                    emoji_data.reactions,
                    emoji_data.in_text,
                    emoji_data.reactions + emoji_data.in_text
                ));
            })
            .await?;
        }
        None => {
            let emojis = db.get_top_emoji_stats(10, ordering).await?;
            ctx.send_embed(|e| {
                e.title("Emoji usage");
                e.description(display_emoji_list(&guild_emojis, emojis.into_iter()));
            })
            .await?;
        }
    }
    Ok(())
}

fn display_emoji_list(
    guildemojis: &HashMap<EmojiId, Emoji>,
    emojis: impl Iterator<Item = EmojiStats>,
) -> String {
    emojis
        .filter_map(|emoji| Some((guildemojis.get(&emoji.emoji.id)?, emoji)))
        .enumerate()
        .map(|(num, (guild_emoji, emoji))| {
            format!(
                "{} {} `{}`: total: {}, reaction: {}, in text: {}",
                num + 1,
                guild_emoji,
                guild_emoji.name,
                emoji.reactions + emoji.in_text,
                emoji.reactions,
                emoji.in_text
            )
        })
        .join("\n")
}
