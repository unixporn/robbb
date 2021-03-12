use itertools::Itertools;
use serenity::futures::{stream, StreamExt};

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

    if !msg.mentions.is_empty() {
        channel_id
            .send_message(&ctx, |m| {
                m.content(format!(
                    "REEEEEEEEEEEEEEEEEEEE {} got 👻-pinged by {}",
                    msg.mentions.iter().map(|x| x.mention()).join(", "),
                    msg.author.id.mention()
                ))
            })
            .await?;
    }

    let channel_name = channel_id
        .name(&ctx)
        .await
        .unwrap_or_else(|| "unknown".to_string());

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let audit_logs = config
        .guild
        .audit_logs(
            &ctx,
            Some(guild::Action::Message(ActionMessage::Delete).num()),
            None,
            None,
            Some(10),
        )
        .await?;

    let deletor = audit_logs
        .entries
        .values()
        .find(|entry| {
            entry.target_id == Some(msg.author.id.0)
                && entry
                    .options
                    .as_ref()
                    .map(|opt| opt.channel_id == Some(channel_id))
                    .unwrap_or(false)
        })
        .map(|entry| entry.user_id)
        .and_then(|deletor| audit_logs.users.iter().find(|usr| usr.id == deletor));

    config
        .channel_bot_messages
        .send_embed(&ctx, |e| {
            e.author(|a| a.name("Message Deleted").icon_url(msg.author.face()));
            e.title(msg.author.name_with_disc_and_id());
            e.description(format!("{}\n\n{}", msg.content, msg.to_context_link()));
            e.footer(|f| {
                f.text(format!(
                    "#{}{}",
                    channel_name,
                    deletor.map_or(Default::default(), |x| format!(", deleted by {}", x.tag()))
                ))
            });
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

    let msgs: Vec<Message> = stream::iter(deleted_message_ids)
        .filter_map(|id| ctx.cache.message(channel_id, id))
        .collect()
        .await;

    let channel_name = channel_id
        .name(&ctx)
        .await
        .unwrap_or_else(|| "unknown".to_string());

    let msg_author = msgs
        .first()
        .context("Could not load any messages from bulk-deletion event")?
        .author
        .clone();

    config
        .channel_bot_messages
        .send_embed(&ctx, |e| {
            e.author(|a| a.name("Message Bulk-deletion").icon_url(msg_author.face()));
            e.title(msg_author.name_with_disc_and_id());
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
