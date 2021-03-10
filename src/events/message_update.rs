use super::*;

pub async fn message_update(
    ctx: client::Context,
    old_if_available: Option<Message>,
    _new: Option<Message>,
    event: MessageUpdateEvent,
) -> Result<()> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();

    if Some(config.guild) != event.guild_id
        || event.edited_timestamp.is_none()
        || event.author.as_ref().map(|x| x.bot).unwrap_or(false)
    {
        return Ok(());
    };

    let msg = event.channel_id.message(&ctx, event.id).await?;

    let channel_name = event
        .channel_id
        .name(&ctx)
        .await
        .unwrap_or_else(|| "unknown".to_string());

    config
        .guild
        .send_embed(&ctx, config.channel_bot_messages, |e| {
            e.author(|a| a.name("Message Edit").icon_url(msg.author.face()));
            e.title(msg.author.name_with_disc_and_id());
            e.description(indoc::formatdoc!(
                "
                        **Before:**
                        {}

                        **Now:**
                        {}

                        [(context)]({})
                    ",
                old_if_available
                    .map(|old| old.content)
                    .unwrap_or_else(|| "<Unavailable>".to_string()),
                event
                    .content
                    .clone()
                    .unwrap_or_else(|| "<Unavailable>".to_string()),
                msg.link()
            ));
            if let Some(edited_timestamp) = event.edited_timestamp {
                e.timestamp(&edited_timestamp);
            }
            e.footer(|f| f.text(format!("#{}", channel_name)));
        })
        .await?;
    Ok(())
}
