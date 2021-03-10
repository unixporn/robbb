use super::*;

pub async fn message(ctx: client::Context, msg: Message) -> Result<()> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    if msg.author.bot {
        return Ok(());
    }
    if msg.channel_id == config.channel_showcase {
        handle_showcase_post(ctx, msg)
            .await
            .context("Failed to handle showcase post")
    } else if msg.channel_id == config.channel_feedback {
        handle_feedback_post(ctx, msg)
            .await
            .context("Failed to handle feedback post")
    } else {
        Ok(())
    }
}

async fn handle_showcase_post(ctx: client::Context, msg: Message) -> Result<()> {
    if msg.attachments.is_empty() {
        msg.delete(&ctx)
            .await
            .context("Failed to delete invalid showcase submission")?;
        msg.author.direct_message(&ctx, |f| {
                f.content(indoc!("
                    Your showcase submission was detected to be invalid. If you wanna comment on a rice, use the #ricing-theming channel.
                    If this is a mistake, contact the moderators or open an issue on https://github.com/unixporn/trup
                "))
            }).await.context("Failed to send DM about invalid showcase submission")?;
    } else {
        msg.react(&ctx, ReactionType::Unicode("❤️".to_string()))
            .await
            .context("Error reacting to showcase submission with ❤️")?;
    }
    Ok(())
}

async fn handle_feedback_post(ctx: client::Context, msg: Message) -> Result<()> {
    msg.react(&ctx, ReactionType::Unicode("👍".to_string()))
        .await
        .context("Error reacting to feedback submission with 👍")?;
    msg.react(&ctx, ReactionType::Unicode("👎".to_string()))
        .await
        .context("Error reacting to feedback submission with 👍")?;
    Ok(())
}
