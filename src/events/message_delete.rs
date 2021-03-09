use super::*;

pub async fn message_delete(
    ctx: client::Context,
    channel_id: ChannelId,
    deleted_message_id: MessageId,
    guild_id: Option<GuildId>,
) -> Result<()> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    if Some(config.guild) != guild_id {
        return Ok(());
    };

    let msg = ctx
        .cache
        .message(channel_id, deleted_message_id)
        .await
        .context("Message not found")?;

    if msg.author.bot {
        return Ok(());
    }

    let channel_name = msg
        .channel_id
        .name(&ctx)
        .await
        .unwrap_or_else(|| "unknown".to_string());

    config
        .channel_bot_messages
        .send_embed(&ctx, |e| {
            e.author(|a| {
                a.name("Message Deleted");
                a.icon_url(msg.author.avatar_or_default())
            });
            e.title(msg.author.name_with_disc_and_id());
            e.description(format!("{}\n\n[(context)]({})", msg.content, msg.link()));
            e.footer(|f| f.text(format!("#{}", channel_name)));
        })
        .await?;
    Ok(())
}
