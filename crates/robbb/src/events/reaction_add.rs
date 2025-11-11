use super::*;

use eyre::ContextCompat as _;
use robbb_db::emoji_logging::EmojiIdentifier;
use serenity::model::channel::ReactionType::Custom;

pub async fn reaction_add(ctx: client::Context, event: Reaction) -> Result<()> {
    let user = event.user(&ctx).await?;
    if user.bot {
        return Ok(());
    }
    let msg = event.message(&ctx).await?;
    if msg.reactions.iter().any(|x| x.reaction_type == event.emoji && x.count == 1) {
        handle_reaction_emoji_logging(ctx, event).await?;
    }
    Ok(())
}

#[tracing::instrument(skip(ctx))]
async fn handle_reaction_emoji_logging(ctx: client::Context, event: Reaction) -> Result<()> {
    let Custom { id, animated, name, .. } = event.emoji else { return Ok(()) };
    let name = name.context("Could not find name for emoji")?;

    let guild_emojis = ctx
        .get_guild_emojis(event.guild_id.context("Not in a guild")?)
        .await
        .context("Could not get guild emojis")?;
    if !guild_emojis.contains_key(&id) {
        return Ok(());
    };

    let db = ctx.get_db().await;
    db.alter_emoji_reaction_count(1, &EmojiIdentifier { animated, id, name }).await?;

    Ok(())
}
