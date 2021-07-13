use std::collections::hash_map::HashMap;

use super::*;

use crate::db::emoji_logging::{EmojiStats, Ordering};

#[command]
#[usage("emojistats [emoji] | --least")]
pub async fn emojistats(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let (ordering, single_emoji) = match args.single_quoted::<String>().ok().as_deref() {
        Some("--least") => (Some(Ordering::Ascending), None),
        Some(arg) => {
            let found_emoji = match crate::util::find_emojis(&arg).first() {
                Some(searched_emoji) => db.get_emoji_usage_by_id(searched_emoji).await,
                None => db.get_emoji_usage_by_name(&arg).await,
            };

            (
                None,
                Some(found_emoji.user_error("Could not find that emote")?),
            )
        }
        None => (Some(Ordering::Descending), None),
    };

    match single_emoji {
        Some(emoji_data) => {
            let guild_emojis = ctx
                .get_guild_emojis(msg.guild_id.context("could not get guild id")?)
                .await
                .context("could not get guild emojis")?;
            let emoji = guild_emojis
                .get(&emoji_data.emoji.id)
                .user_error("Could not find emoji in guild")?;
            msg.reply_embed(ctx, |e| {
                e.title(format!("Emoji usage for *{}*", emoji.name));
                e.image(emoji.url());
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
            let emojis = db
                .get_top_emoji_stats(
                    10,
                    ordering.context("no ordering is found for some reason")?,
                )
                .await?;
            let guild_emojis = ctx
                .get_guild_emojis(msg.guild_id.context("could not get guild id")?)
                .await
                .context("could not guild emojis")?;
            msg.reply_embed(ctx, |e| {
                e.title("Emoji usage");
                e.description(display_emoji_list(&guild_emojis, emojis));
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
        .enumerate()
        .filter_map(|(num, emoji)| {
            let guild_emoji = guildemojis.get(&emoji.emoji.id)?;
            Some(format!(
                "{} {} `{}`: total: {}, reaction: {}, in text: {}",
                num + 1,
                guild_emoji,
                guild_emoji.name,
                emoji.reactions + emoji.in_text,
                emoji.reactions,
                emoji.in_text
            ))
        })
        .join("\n")
}
