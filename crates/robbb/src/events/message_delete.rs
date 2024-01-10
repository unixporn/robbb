use futures::StreamExt;
use itertools::Itertools;
use poise::serenity_prelude::{AuditLogEntry, MessageAction};
use robbb_util::embeds;
use serenity::{
    all::audit_log,
    builder::{CreateAttachment, CreateEmbed, CreateEmbedFooter, CreateMessage, GetMessages},
};

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

    let msg = ctx.cache.message(channel_id, deleted_message_id).map(|x| x.to_owned());
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
            .messages(&ctx, GetMessages::default().after(deleted_message_id).limit(5))
            .await?;
        let bot_reply = close_messages.iter().find(|x| {
            x.message_reference.as_ref().and_then(|x| x.message_id) == Some(deleted_message_id)
                && x.author.bot
        });
        if let Some(bot_reply) = bot_reply {
            log_error!(bot_reply.delete(&ctx).await);
        }
    }

    let deletor = find_deletor(&ctx, &config, &msg).await?;
    let channel_name =
        util::channel_name(&ctx, channel_id).await.unwrap_or_else(|_| "unknown".to_string());

    let attachments: Vec<_> = futures::stream::iter(attachments.iter())
        .then(|(path, file)| {
            CreateAttachment::file(
                file,
                path.file_name().and_then(|x| x.to_str()).unwrap_or("attachment").to_string(),
            )
        })
        .collect()
        .await;
    let embed = CreateEmbed::default()
        .author_icon("Message Deleted", msg.author.face())
        .title(msg.author.name_with_disc_and_id())
        .description(format!("{}\n\n{}", msg.content, msg.to_context_link()))
        .footer(CreateEmbedFooter::new(format!(
            "#{}{}",
            channel_name,
            deletor.map_or_else(String::new, |x| format!(", deleted by {}", x.tag()))
        )));

    config
        .channel_bot_messages
        .send_message(
            &ctx,
            CreateMessage::default()
                .add_files(attachments.into_iter().filter_map(|x| x.ok()))
                .embed(embed),
        )
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
        message_delete(ctx, channel_id, deleted_message_ids.pop().unwrap(), guild_id).await?;
        return Ok(());
    }

    // Channel the messages where in
    let channel_name = channel_id.name(&ctx).await.unwrap_or_else(|_| "unknown".to_string());

    // Look through the cache to try to find the messages that where just deleted
    let msgs: Vec<_> = deleted_message_ids
        .iter()
        .filter_map(|id| ctx.cache.message(channel_id, id))
        .map(|x| x.to_owned())
        .collect();

    if msgs.is_empty() {
        config
            .channel_bot_messages
            .send_embed_builder(&ctx, |e| {
                e.title("Message bulk-deletion")
                .description(format!(
                    "Messages where bulk-deleted in {}. Sadly, I don't remember any of these messages :(",
                    channel_id.mention()
                ))
                .footer(CreateEmbedFooter::new(format!("#{channel_name}")))
            })
            .await?;
    } else {
        // Author of the deleted messages
        let msg_author = msgs
            .first()
            .context("Could not find any messages from bulk-deletion event in cache")?
            .author
            .clone();

        config
            .channel_bot_messages
            .send_embed(
                &ctx,
                embeds::base_embed(&ctx)
                    .await
                    .author_icon("Message Bulk-deletion", msg_author.face())
                    .title(msg_author.name_with_disc_and_id())
                    .description(
                        msgs.into_iter()
                            .map(|m| {
                                format!("[{}]\n{}\n", util::format_date(*m.timestamp), m.content)
                            })
                            .join("\n"),
                    )
                    .footer(CreateEmbedFooter::new(format!("#{channel_name}"))),
            )
            .await?;
    }
    Ok(())
}

/// Polls the audit log a few times, trying to figure out who deleted the given message
async fn find_deletor(
    ctx: &client::Context,
    config: &Config,
    msg: &Message,
) -> Result<Option<User>> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let result = await_audit_log(
        ctx,
        &config.guild,
        audit_log::Action::Message(MessageAction::Delete),
        None,
        |entry| {
            entry.target_id.map(|x| x.get()) == Some(msg.author.id.get())
                && entry
                    .options
                    .as_ref()
                    .map_or(false, |opt| opt.channel_id == Some(msg.channel_id))
        },
    )
    .await?;
    if let Some((entry, users)) = result {
        Ok(users.get(&entry.user_id).cloned())
    } else {
        Ok(None)
    }
}

async fn await_audit_log(
    ctx: &client::Context,
    guild: &GuildId,
    action_type: audit_log::Action,
    user_id: Option<UserId>,
    filter: impl Fn(&AuditLogEntry) -> bool,
) -> Result<Option<(AuditLogEntry, std::collections::HashMap<UserId, User>)>> {
    for _ in 0..3 {
        let results = guild.audit_logs(&ctx, Some(action_type), user_id, None, None).await?;
        let matching_value = results.entries.into_iter().find(|x| filter(x));
        if let Some(matching_value) = matching_value {
            return Ok(Some((matching_value, results.users)));
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    Ok(None)
}
