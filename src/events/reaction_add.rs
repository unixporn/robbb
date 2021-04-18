use super::*;

use serenity::model::channel::ReactionType::Custom;

pub async fn reaction_add(ctx: client::Context, event: Reaction) -> Result<()> {
    let user = event.user(&ctx).await?;
    if user.bot {
        return Ok(());
    }
    let msg = event.message(&ctx).await?;

    let is_poll = msg.author.bot
        && msg.embeds.iter().any(|embed| {
            embed
                .title
                .as_ref()
                .map(|x| x.starts_with("Poll"))
                .unwrap_or(false)
        });

    if is_poll {
        // This is rather imperfect, but discord API sucks :/
        // we're pretty much deleteing all other reactions and are giving it the user to delete the reaction from,
        // such that discord API knows which of the reactions to remove. If the user hasn't reacted
        // with that emote, it'll error, but we don't really care :/
        for r in msg.reactions {
            if r.reaction_type != event.emoji {
                crate::log_error!(
                    ctx.http
                        .delete_reaction(
                            msg.channel_id.0,
                            msg.id.0,
                            Some(user.id.0),
                            &r.reaction_type
                        )
                        .await
                );
            }
        }
    }
    handle_emoji_logging(ctx, event).await?;
    Ok(())
}

async fn handle_emoji_logging(ctx: client::Context, event: Reaction) -> Result<()> {
    let id = match event.emoji {
        Custom { id, .. } => id,
        _ => return Ok(()),
    };

    let emoji = ctx.http.get_emojis(event.guild_id.unwrap().0).await?;
    let emoji = match emoji.iter().find(|x| x.id == id) {
        Some(x) => x,
        _ => return Ok(()),
    };
    let data = ctx.data.read().await;

    let db = data.get::<Db>().unwrap();
    db.increment_emoji_reaction(
        1,
        &EmojiIdentifier {
            name: emoji.name.clone(),
            id: emoji.id,
            animated: emoji.animated,
        },
    )
    .await?;

    Ok(())
}
