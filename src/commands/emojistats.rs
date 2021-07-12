use std::collections::hash_map::HashMap;

use super::*;

use crate::db::emoji_logging::EmojiStats;

#[command]
#[usage("emojistats [in_text or reactions]")]
#[usage("emojistats [emoji] ")]
#[usage("emojistats [emoji] [in_text or reactions]")]
pub async fn emojistats(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let single_emoji = match args.single_quoted::<String>().ok() {
        Some(x) if !crate::util::find_emojis(&x).is_empty() => Some(
            db.get_emoji_usage(
                crate::util::find_emojis(&x)
                    .first()
                    .user_error(format!("Could not find emoji {}", x).as_str())?,
            )
            .await?,
        ),
        Some(x) => Some(db.get_emoji_usage_name(&x).await?),
        None => None,
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
                    "**Reactions:** {} \n**In Text:** {} \n**Both:** {}",
                    emoji_data.reactions,
                    emoji_data.in_text,
                    emoji_data.reactions + emoji_data.in_text
                ));
            })
            .await?;
        }
        None => {
            let emojis = db.get_ordered_emojis(10).await?;
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
                "{} {} `{}`: total: {}, reaction: {}, in text: {}
",
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
