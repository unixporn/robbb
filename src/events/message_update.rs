use super::*;

#[tracing::instrument(skip_all, fields(msg.id = %event.id, msg.channel_id = %event.channel_id, ?event))]
pub async fn message_update(
    ctx: client::Context,
    _data: UserData,
    old_if_available: Option<Message>,
    _new: Option<Message>,
    event: MessageUpdateEvent,
) -> Result<()> {
    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();
    let config = ctx.get_config().await;

    if Some(config.guild) != event.guild_id
        || event.edited_timestamp.is_none()
        || event.author.as_ref().map(|x| x.bot).unwrap_or(false)
    {
        return Ok(());
    };

    let mut msg = event.channel_id.message(&ctx, event.id).await?;
    msg.guild_id = event.guild_id;

    match handle_blocklist::handle_blocklist(&ctx, &msg).await {
        Ok(false) => {}
        err => log_error!("error while handling blocklist in message_update", err),
    };

    let channel_name = util::channel_name(&ctx, event.channel_id)
        .await
        .unwrap_or_else(|_| "unknown".to_string());

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

                        {}
                    ",
                old_if_available
                    .map(|old| old.content)
                    .unwrap_or_else(|| "<Unavailable>".to_string()),
                event
                    .content
                    .clone()
                    .unwrap_or_else(|| "<Unavailable>".to_string()),
                msg.to_context_link()
            ));
            if let Some(edited_timestamp) = event.edited_timestamp {
                e.timestamp(&edited_timestamp);
            }
            e.footer(|f| f.text(format!("#{}", channel_name)));
        })
        .await?;
    Ok(())
}
