use super::*;

use serenity::model::channel::ReactionType::Custom;

pub async fn reaction_add(ctx: client::Context, event: Reaction) -> Result<()> {
    let user = event.user(&ctx).await?;
    if user.bot {
        return Ok(());
    }
    let msg = event.message(&ctx).await?;

    let is_poll = msg.author.bot
        && msg
            .embeds
            .iter()
            .any(|embed| embed.title.as_ref().map(|x| x.starts_with("Poll")).unwrap_or(false));

    if is_poll {
        // This is rather imperfect, but discord API sucks :/
        // we're pretty much deleteing all other reactions and are giving it the user to delete the reaction from,
        // such that discord API knows which of the reactions to remove. If the user hasn't reacted
        // with that emote, it'll error, but we don't really care :/
        for r in &msg.reactions {
            if r.reaction_type != event.emoji {
                tracing::debug!(reaction.typ = ?r.reaction_type, reaction.user_id = %user.id, msg.id = %msg.id, "Removing reaction to poll");
                log_error!(msg.delete_reaction(&ctx, Some(user.id), r.reaction_type.clone()).await);
            }
        }
    }
    if !is_poll && msg.reactions.iter().any(|x| x.reaction_type == event.emoji && x.count == 1) {
        handle_reaction_emoji_logging(ctx, event).await?;
    }
    Ok(())
}

#[tracing::instrument(skip(ctx))]
async fn handle_reaction_emoji_logging(ctx: client::Context, event: Reaction) -> Result<()> {
    let (id, animated, name) = match event.emoji {
        Custom { id, animated, name, .. } => {
            (id, animated, name.context("Could not find name for emoji")?)
        }
        _ => return Ok(()),
    };

    let guild_emojis = ctx
        .get_guild_emojis(event.guild_id.context("Not in a guild")?)
        .await
        .context("Could not get guild emojis")?;
    if !guild_emojis.contains_key(&id) {
        return Ok(());
    };

    let db = ctx.get_db().await;
    db.alter_emoji_reaction_count(
        1,
        &robbb_db::emoji_logging::EmojiIdentifier { animated, id, name },
    )
    .await?;

    Ok(())
}
