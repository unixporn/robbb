use std::collections::hash_map::HashMap;

use super::*;

use crate::db::emoji_logging::EmojiData;

#[command]
#[usage("emojistats [in_text or reactions]")]
#[usage("emojistats [emoji] ")]
#[usage("emojistats [emoji] [in_text or reactions]")]
pub async fn emojistats(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.context("Could not get guild")?;
    let db: std::sync::Arc<Db> = ctx.get_db().await;
    let guild_emojis = guild.emojis;
    let emojis = db.get_all_emojis().await?.collect_vec();

    let single_emoji = match args.single_quoted::<String>().ok() {
        Some(x) if !crate::util::find_emojis(&x).is_empty() => Some(
            emojis
                .iter()
                .find(|y| {
                    crate::util::find_emojis(&x)
                        .first()
                        .map(|x| x == &y.emoji)
                        .unwrap_or(false)
                })
                .user_error("Could not find emoji x")?,
        ),
        Some(x) => Some(
            emojis
                .iter()
                .find(|e| e.emoji.name == x)
                .user_error("Could not find emoji of that name")?,
        ),
        None => None,
    };

    match single_emoji {
        Some(emoji_data) => {
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
            let emojis = emojis.into_iter();
            msg.reply_embed(ctx, |e| {
                e.title("Emoji usage");
                e.description(display_emojis(&guild_emojis, emojis));
            })
            .await?;
        }
    }
    Ok(())
}

fn display_emojis(
    servemojis: &HashMap<EmojiId, Emoji>,
    emojis: impl Iterator<Item = EmojiData>,
) -> String {
    emojis
        .enumerate()
        .map(|(num, emoji)| {
            let print_emoji = servemojis
                .get(&emoji.emoji.id)
                .context("Emoji id could not be found")
                .unwrap();
            format!(
                "**{}**. {} {} {} ({})",
                num + 1,
                print_emoji.name,
                print_emoji,
                if print_emoji.animated { "animated" } else { "" },
                emoji.reactions + emoji.in_text
            )
        })
        .join("\n")
}
