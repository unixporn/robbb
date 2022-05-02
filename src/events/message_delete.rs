use itertools::Itertools;

use super::*;

pub async fn message_delete(
    ctx: client::Context,
    channel_id: ChannelId,
    deleted_message_id: MessageId,
    guild_id: Option<GuildId>,
) -> Result<()> {
    let config = ctx.get_config().await;
    if Some(config.guild) != guild_id {
        return Ok(());
    };

    let attachments = crate::attachment_logging::find_attachments_for(
        &config.attachment_cache_path,
        channel_id,
        deleted_message_id,
    )
    .await?;

    let msg = ctx.cache.message(channel_id, deleted_message_id);
    // if the message can't be loaded, there's no need to try anything more,
    // so let's just give up. No need to error.
    let msg = match msg {
        Some(msg) => msg,
        None => return Ok(()),
    };

    if msg.author.bot {
        return Ok(());
    }

    if msg.content.starts_with('!') {
        let close_messages = msg
            .channel_id
            .messages(&ctx, |m| m.after(deleted_message_id).limit(5))
            .await?;
        let bot_reply = close_messages.iter().find(|x| {
            x.message_reference.as_ref().and_then(|x| x.message_id) == Some(deleted_message_id)
                && x.author.bot
        });
        if let Some(bot_reply) = bot_reply {
            log_error!(bot_reply.delete(&ctx).await);
        }
    }

    // TODO do we want this?
    //handle_ghostping(&ctx, &msg).await;

    let deletor = find_deletor(&ctx, &config, &msg).await?;
    let channel_name = util::channel_name(&ctx, channel_id)
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    config
        .channel_bot_messages
        .send_message(&ctx, |m| {
            m.add_files(attachments.iter().map(|(path, file)| {
                AttachmentType::File {
                    filename: path
                        .file_name()
                        .and_then(|x| x.to_str())
                        .unwrap_or("attachment")
                        .to_string(),
                    file,
                }
            }));
            m.embed(|e| {
                e.author(|a| a.name("Message Deleted").icon_url(msg.author.face()));
                e.title(msg.author.name_with_disc_and_id());
                e.description(format!("{}\n\n{}", msg.content, msg.to_context_link()));
                e.footer(|f| {
                    f.text(format!(
                        "#{}{}",
                        channel_name,
                        deletor.map_or(Default::default(), |x| format!(", deleted by {}", x.tag()))
                    ))
                })
            })
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
    let config = ctx.get_config().await;
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

    let msgs: Vec<Message> = deleted_message_ids
        .iter()
        .filter_map(|id| ctx.cache.message(channel_id, id))
        .collect();

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
                    .map(|m| format!("[{}]\n{}\n", util::format_date(*m.timestamp), m.content))
                    .join("\n"),
            );
            e.footer(|f| f.text(format!("#{}", channel_name)));
        })
        .await?;
    Ok(())
}

/// waits a second, then looks through the audit log, trying to find who deleted the given message.
async fn find_deletor(
    ctx: &client::Context,
    config: &Config,
    msg: &Message,
) -> Result<Option<User>> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let audit_logs = config
        .guild
        .audit_logs(
            &ctx,
            Some(guild::Action::Message(MessageAction::Delete).num()),
            None,
            None,
            Some(10),
        )
        .await?;

    Ok(audit_logs
        .entries
        .iter()
        .find(|entry| {
            entry.target_id == Some(msg.author.id.0)
                && entry
                    .options
                    .as_ref()
                    .map(|opt| opt.channel_id == Some(msg.channel_id))
                    .unwrap_or(false)
        })
        .map(|entry| entry.user_id)
        .and_then(|deletor| {
            audit_logs
                .users
                .iter()
                .find(|(id, _)| **id == deletor)
                .map(|(_, usr)| usr)
        })
        .cloned())
}

/// if the deleted message was a ghostping, send a message to the pinged user.
#[allow(unused)]
async fn handle_ghostping(ctx: &client::Context, msg: &Message) {
    // TODO make this not fire in showcase etc
    if !msg.mentions.is_empty() {
        let _ = msg
            .channel_id
            .send_message(&ctx, |m| {
                m.content(format!(
                    "REEEEEEEEEEEEEEEEEEEE {} got 👻-pinged by {}",
                    msg.mentions.iter().map(|x| x.mention()).join(", "),
                    msg.author.id.mention()
                ))
            })
            .await;
    }
}
