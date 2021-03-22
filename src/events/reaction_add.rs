use super::*;

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
        // reactions that are not the one that was just added
        let other_reactions = msg
            .reactions
            .iter()
            .filter(|r| r.reaction_type != event.emoji);

        // This is rather imperfect, but discord API sucks :/
        // we're pretty much deleteing all other reactions and are giving it the user to delete the reaction from,
        // such that discord API knows which of the reactions to remove. If the user hasn't reacted
        // with that emote, it'll error, but we don't really care :/
        for r in other_reactions {
            crate::log_error!(
                ctx.http
                    .delete_reaction(
                        msg.channel_id.0,
                        msg.id.0,
                        Some(user.id.0),
                        &r.reaction_type,
                    )
                    .await
            )
        }
    }
    Ok(())
}
