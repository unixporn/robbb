use poise::serenity_prelude::MessageUpdateEvent;

use super::*;

pub async fn message_update(
    ctx: &client::Context,
    old_if_available: Option<Message>,
    new: Option<Message>,
    event: MessageUpdateEvent,
) -> Result<()> {
    let config = ctx.get_config().await;

    if Some(config.guild) != event.guild_id
        || event.edited_timestamp.is_none()
        || event.author.as_ref().map(|x| x.bot).unwrap_or(false)
    {
        return Ok(());
    };

    let old_content = old_if_available
        .as_ref()
        .map(|x| x.content.to_string())
        .unwrap_or_else(|| "<Unavailable>".to_string());

    if let Some(new) = new {
        let channel_name =
            new.channel_id.name(ctx).await.unwrap_or_else(|_| "unknown-name".to_string());
        tracing::info!(
            msg.id = %event.id,
            msg.author = %new.author.tag(),
            msg.author_id = %new.author.id,
            msg.channel = %channel_name,
            msg.channel_id = %new.channel_id,
            msg.content = %new.content,
            msg.old_content = %old_content,
            "handling message_update event"
        );
    } else {
        tracing::info!(msg.id = %event.id, "handling message_update event");
    }

    let mut msg = event.channel_id.message(&ctx, event.id).await?;
    msg.guild_id = event.guild_id;

    match handle_blocklist::handle_blocklist(&ctx, &msg).await {
        Ok(false) => {}
        err => log_error!("error while handling blocklist in message_update", err),
    };

    let channel_name =
        util::channel_name(&ctx, event.channel_id).await.unwrap_or_else(|_| "unknown".to_string());

    config
        .guild
        .send_embed(&ctx, config.channel_bot_messages, |e| {
            e.timestamp_opt(event.edited_timestamp)
                .author_icon("Message Edit", msg.author.face())
                .title(msg.author.name_with_disc_and_id())
                .description(indoc::formatdoc!(
                    "
                        **Before:**
                        {}

                        **Now:**
                        {}

                        {}
                    ",
                    old_content,
                    event.content.clone().unwrap_or_else(|| "<Unavailable>".to_string()),
                    msg.to_context_link()
                ))
                .footer_str(format!("#{channel_name}"))
        })
        .await?;
    Ok(())
}
