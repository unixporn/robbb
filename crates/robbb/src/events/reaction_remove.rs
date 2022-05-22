use super::*;

use serenity::model::channel::ReactionType::Custom;

pub async fn reaction_remove(ctx: client::Context, event: Reaction) -> Result<()> {
    let user = event.user(&ctx).await?;
    if user.bot {
        return Ok(());
    }

    let msg = event.message(&ctx).await?;
    if !msg.reactions.iter().any(|x| x.reaction_type == event.emoji) {
        handle_emoji_removal(ctx, event).await?;
    }
    Ok(())
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_emoji_removal(ctx: client::Context, event: Reaction) -> Result<()> {
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
    db.alter_emoji_reaction_count(-1, &EmojiIdentifier { animated, id, name }).await?;

    Ok(())
}
