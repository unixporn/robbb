use std::time::Duration;

use eyre::ContextCompat as _;
use futures::StreamExt;
use itertools::Itertools;
use poise::serenity_prelude::MessageAction;
use robbb_util::embeds;
use serenity::{
    all::audit_log,
    builder::{CreateAttachment, CreateEmbed, CreateMessage},
};

use super::*;

pub async fn message_delete(
    ctx: &client::Context,
    channel_id: ChannelId,
    deleted_message_id: MessageId,
    guild_id: Option<GuildId>,
) -> Result<()> {
    let config = ctx.get_config().await;
    if Some(config.guild) != guild_id {
        return Ok(());
    };

    tracing::info!(msg.id = %deleted_message_id, msg.channel_id = %channel_id, "Handling message_delete event");

    let attachments = crate::attachment_logging::find_attachments_for(
        &config.attachment_cache_path,
        channel_id,
        deleted_message_id,
    )
    .await?;

    let msg = ctx.cache.message(channel_id, deleted_message_id).map(|x| x.to_owned());
    // if the message can't be loaded, there's no need to try anything more,
    // so let's just give up. No need to error.
    let Some(msg) = msg else {
        return Ok(());
    };

    if msg.author.bot {
        return Ok(());
    }

    let channel_name = channel_id.name(&ctx).await.unwrap_or_else(|_| "unknown".to_string());

    tracing::info!(
        msg.id = %deleted_message_id,
        msg.channel_id = %channel_id,
        msg.channel = %channel_name,
        msg.author = %msg.author.tag(),
        msg.author_id = %msg.author.id,
        msg.content = %msg.content,
        "Found deleted message in cache"
    );

    let deletor = find_deletor(&ctx, &config, &msg).await?;

    let attachments: Vec<_> = futures::stream::iter(attachments.iter())
        .then(|(path, file)| {
            CreateAttachment::file(
                file,
                path.file_name().and_then(|x| x.to_str()).unwrap_or("attachment").to_string(),
            )
        })
        .collect()
        .await;
    let deletor_str = deletor.map(|x| format!(", deleted by {}", x.tag())).unwrap_or_default();
    let embed = CreateEmbed::default()
        .author_icon("Message Deleted", msg.author.face())
        .title(msg.author.name_with_disc_and_id())
        .description(format!("{}\n\n{}", msg.content, msg.to_context_link()))
        .footer_str(format!("#{channel_name}{deletor_str}"));

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
        message_delete(&ctx, channel_id, deleted_message_ids.pop().unwrap(), guild_id).await?;
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
                    "Messages were bulk-deleted in {}. Sadly, I don't remember any of these messages :(",
                    channel_id.mention()
                ))
                .footer_str(format!("#{channel_name}"))
            })
            .await?;
    } else {
        // Author of the deleted messages
        let msg_author = msgs
            .first()
            .context("Could not find any messages from bulk-deletion event in cache")?
            .author
            .clone();
        let embed = embeds::base_embed_ctx(&ctx)
            .await
            .author_icon("Message Bulk-deletion", msg_author.face())
            .title(msg_author.name_with_disc_and_id())
            .description(
                msgs.into_iter()
                    .map(|m| format!("[{}]\n{}\n", util::format_date(*m.timestamp), m.content))
                    .join("\n"),
            )
            .footer_str(format!("#{channel_name}"));

        config.channel_bot_messages.send_embed(&ctx, embed).await?;
    }
    Ok(())
}

/// Looks up who deleted `msg` by first checking the short-lived audit-log cache
/// (populated reactively via `guild_audit_log_entry_create`) and, only if that
/// misses, falling back to a single REST poll. This replaces a previous design
/// that polled up to three times and suffered from a wrong filter (comparing
/// `entry.target_id` / the author's UserId against `msg.id` / the MessageId).
async fn find_deletor(
    ctx: &client::Context,
    config: &Config,
    msg: &Message,
) -> Result<Option<User>> {
    let cache = ctx.get_deletion_audit_cache().await;

    // Give the GUILD_AUDIT_LOG_ENTRY_CREATE gateway event time to arrive
    // and be processed before we check the cache.
    tokio::time::sleep(Duration::from_secs(5)).await;

    if let Some(deleter_id) = cache.get(msg.channel_id, msg.author.id) {
        return Ok(Some(deleter_id.to_user(ctx).await?));
    }

    // Fallback: one direct audit-log REST call for cases where the gateway
    // event is delayed or never arrives (e.g. older Discord clients).
    tracing::debug!(
        msg.channel_id = %msg.channel_id,
        msg.author = %msg.author.id,
        "Deletion not in audit cache, falling back to single audit log poll"
    );
    let results = config
        .guild
        .audit_logs(ctx, Some(audit_log::Action::Message(MessageAction::Delete)), None, None, None)
        .await?;

    let entries = results.entries;
    let users = results.users;
    let matching = entries.into_iter().find(|entry| {
        // target_id for MESSAGE_DELETE is the author's UserId, not the MessageId.
        entry.target_id.map(|x| x.get()) == Some(msg.author.id.get())
            && entry.options.as_ref().is_some_and(|opt| opt.channel_id == Some(msg.channel_id))
    });

    Ok(matching.and_then(|entry| users.get(&entry.user_id).cloned()))
}
