use anyhow::bail;
use itertools::Itertools;

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

    let channel_name = channel_id
        .name(&ctx)
        .await
        .unwrap_or_else(|| "unknown".to_string());

    config
        .channel_bot_messages
        .send_embed(&ctx, |e| {
            e.author(|a| a.name("Message Deleted").icon_url(msg.author.face()));
            e.title(msg.author.name_with_disc_and_id());
            e.description(format!("{}\n\n{}", msg.content, msg.to_context_link()));
            e.footer(|f| f.text(format!("#{}", channel_name)));
        })
        .await?;
    Ok(())
}

pub async fn message_delete_bulk(
    ctx: client::Context,
    channel_id: ChannelId,
    deleted_message_ids: Vec<MessageId>,
    guild_id: Option<GuildId>,
) -> Result<()> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    if Some(config.guild) != guild_id {
        return Ok(());
    };

    if deleted_message_ids.len() == 1 {
        let mut deleted_message_ids = deleted_message_ids;
        message_delete(
            ctx,
            channel_id,
            deleted_message_ids.pop().unwrap(),
            guild_id,
        )
        .await?;
        return Ok(());
    }

    let mut msgs = Vec::new();
    for id in deleted_message_ids {
        if let Some(msg) = ctx.cache.message(channel_id, id).await {
            msgs.push(msg);
        };
    }

    if msgs.is_empty() {
        bail!("Could not load any messages from bulk-deletion event");
    }

    let channel_name = channel_id
        .name(&ctx)
        .await
        .unwrap_or_else(|| "unknown".to_string());

    config
        .channel_bot_messages
        .send_embed(&ctx, |e| {
            e.author(|a| {
                a.name("Message Bulk-deletion")
                    .icon_url(msgs.first().unwrap().author.face())
            });
            e.title(msgs.first().unwrap().author.name_with_disc_and_id());
            e.description(
                msgs.into_iter()
                    .map(|m| format!("[{}]\n{}\n", util::format_date(m.timestamp), m.content))
                    .join("\n"),
            );
            e.footer(|f| f.text(format!("#{}", channel_name)));
        })
        .await?;
    Ok(())
}
