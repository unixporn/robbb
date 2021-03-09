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
        // This is rather imperfect, but discord API sucks :/
        serenity::futures::future::join_all(
            msg.reactions
                .iter()
                .filter(|r| r.reaction_type != event.emoji)
                .map(|r| {
                    ctx.http.delete_reaction(
                        msg.channel_id.0,
                        msg.id.0,
                        Some(user.id.0),
                        &r.reaction_type,
                    )
                }),
        )
        .await;
    }
    Ok(())
}
