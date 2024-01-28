use std::collections::HashSet;

use chrono::Utc;
use itertools::Itertools;
use maplit::hashmap;
use poise::serenity_prelude::{MessageType, ReactionType};
use regex::Regex;
use robbb_commands::{commands, modlog};
use robbb_db::emoji_logging::EmojiIdentifier;
use robbb_db::fetch_field::FetchField;

use robbb_util::cdn_hack;
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage, GetMessages};
use tracing::debug;
use tracing_futures::Instrument;

use crate::attachment_logging;

use super::*;

/// Handle a message-create event. If this returns `Ok(true)`,
/// the message should _not_ be forwarded to the command framework.
/// Otherwise, it should be forwarded.
pub async fn message_create(ctx: client::Context, msg: Message) -> Result<bool> {
    let config = ctx.get_config().await;

    if msg.author.bot {
        return Ok(true);
    }

    tracing::debug!(
        msg.content = %msg.content,
        msg.author = %msg.author.tag(),
        msg.author_id = %msg.author.id,
        msg.id = %msg.id,
        msg.channel_id = %msg.channel_id,
        "new message from {}: {}",
        msg.author.tag(),
        msg.content
    );

    handle_attachment_logging(&ctx, &msg).await;

    if msg.channel_id == config.channel_showcase {
        log_error!(handle_showcase_post(&ctx, &msg).await);
    } else if msg.channel_id == config.channel_feedback {
        log_error!(handle_feedback_post(&ctx, &msg).await);
    }

    if !msg.is_private() && msg.channel_id != config.channel_bot_messages {
        match handle_msg_emoji_logging(&ctx, &msg).await {
            Ok(emoji_used) => {
                tracing::Span::current().record("message_create.emoji_used", emoji_used);
            }
            err => log_error!("Error while handling emoji logging", err),
        }
    }

    let (stop_after_spam_protect, stop_after_blocklist) = tokio::join!(
        handle_spam_protect(&ctx, &msg),
        handle_blocklist::handle_blocklist(&ctx, &msg),
    );

    match stop_after_spam_protect {
        Ok(stop) if stop => {
            tracing::info!("Stopping message handling after spam protection");
            return Ok(true);
        }
        err => log_error!("error while handling spam-protection", err),
    };

    match stop_after_blocklist {
        Ok(stop) if stop => {
            tracing::info!("Stopping message handling after blocklist handling");
            return Ok(true);
        }
        err => log_error!("error while handling blocklist", err),
    };

    let (highlighting_notified_users, quoting_result) =
        tokio::join!(handle_highlighting(&ctx, &msg), handle_quote(&ctx, &msg));

    match highlighting_notified_users {
        Ok(notified_users) => {
            tracing::Span::current().record("message_create.notified_user_cnt", notified_users);
        }
        err => log_error!("error while checking/handling highlights", err),
    }

    log_error!("error while Handling a quoted message", quoting_result);

    // If the message is in showcase, don't forward to the command framework
    Ok(msg.channel_id == config.channel_showcase)
}

#[tracing::instrument(skip_all, fields(highlights.notified_user_cnt))]
async fn handle_highlighting(ctx: &client::Context, msg: &Message) -> Result<usize> {
    let (config, db) = ctx.get_config_and_db().await;

    let highlights_data = db.get_highlights().await?;

    let highlight_matches = tokio::task::spawn_blocking({
        let msg_content = msg.content.to_string();
        move || highlights_data.get_triggers_for_message(&msg_content)
    })
    .instrument(tracing::debug_span!("highlights-trigger-check"))
    .await
    .context("Failed to get highlight triggers for a message")?;

    if highlight_matches.is_empty() {
        tracing::Span::current().record("highlights.notified_user_cnt", 0i32);
        return Ok(0);
    }
    // don't highlight in threads or mod internal channels
    // We do this after checking for highlights as checking for highlights is a lot
    // cheaper than potentially sending discord API requests for
    // a lot of messages, specifically in threads
    let channel = msg
        .channel(&ctx)
        .await
        .context("Couldn't get channel")?
        .guild()
        .context("Couldn't get a guild-channel from the channel")?;
    if channel.thread_metadata.is_some()
        || config.category_mod_private == channel.parent_id.context("Couldn't get category_id")?
    {
        tracing::Span::current().record("highlights.notified_user_cnt", 0i32);
        return Ok(0);
    }

    let mut handled_users = HashSet::new();
    for (word, users) in highlight_matches {
        let embed = CreateEmbed::default()
            .title("Highlight notification")
            .description(indoc::formatdoc!(
                "`{word}` has been mentioned in {}
                [link to message]({})

                Don't care about this anymore?
                Run `!highlights remove {word}` in #bot to stop getting these notifications.",
                msg.channel_id.mention(),
                msg.link(),
            ))
            .author_user(&msg.author)
            .timestamp(msg.timestamp)
            .footer_str(format!("#{}", channel.name));

        tracing::debug!(
            highlights.word = %word,
            highlights.users = ?users,
            highlights.users_count = %users.len(),
            "Notifying {} users about a mention of the word '{word}'", users.len()
        );

        let create_message = embed.into_create_message();
        for user_id in users {
            let user_can_see_channel = channel
                .guild_id
                .member(&ctx, msg.author.id)
                .await?
                .permissions(ctx)?
                .read_message_history();

            if user_id == msg.author.id
                // check if the user has already been notified of another word in this message
                || handled_users.contains(&user_id)
                // check if the user can read that channel
                || !user_can_see_channel
            {
                continue;
            }
            handled_users.insert(user_id);

            let ctx = ctx.clone();
            let create_message = create_message.clone();
            tokio::spawn(async move {
                if let Ok(dm_channel) = user_id.create_dm_channel(&ctx).await {
                    let _ = dm_channel.send_message(&ctx, create_message).await;
                }
            })
            .instrument(tracing::info_span!("send_highlight_notification"));
        }
    }

    tracing::Span::current().record("highlights.notified_user_cnt", handled_users.len());
    Ok(handled_users.len())
}

#[tracing::instrument(skip_all, fields(msg_emoji_logging.emoji_used, guild_id = ?msg.guild_id))]
async fn handle_msg_emoji_logging(ctx: &client::Context, msg: &Message) -> Result<usize> {
    let actual_emojis = util::find_emojis(&msg.content);
    if actual_emojis.is_empty() {
        return Ok(0);
    }
    let guild_emojis = ctx
        .get_guild_emojis(msg.guild_id.context("could not get guild")?)
        .await
        .context("could not get emojis for guild")?;

    let actual_emojis = tracing::debug_span!("find_guild_emojis_in_message", %msg.content)
        .in_scope(|| {
            actual_emojis
                .into_iter()
                .filter(|iden| guild_emojis.contains_key(&iden.id))
                .dedup_by(|x, y| x.id == y.id)
                .collect_vec()
        });

    if actual_emojis.is_empty() {
        return Ok(0);
    }
    let db = ctx.get_db().await;
    for emoji in &actual_emojis {
        db.alter_emoji_text_count(
            1,
            &EmojiIdentifier { name: emoji.name.clone(), id: emoji.id, animated: emoji.animated },
        )
        .await?;
    }
    tracing::Span::current().record("msg_emoji_logging.emoji_used", actual_emojis.len());
    Ok(actual_emojis.len())
}

#[tracing::instrument(skip_all)]
async fn handle_attachment_logging(ctx: &client::Context, msg: &Message) {
    if msg.attachments.is_empty() {
        return;
    }
    let config = ctx.get_config().await;

    let msg_id = msg.id;
    let channel_id = msg.channel_id;

    let attachments = msg.attachments.clone();
    tokio::spawn(
        async move {
            log_error!(
                "Storing attachments in message",
                attachment_logging::store_attachments(
                    attachments,
                    msg_id,
                    channel_id,
                    config.attachment_cache_path.clone(),
                )
                .await,
            )
        }
        .instrument(tracing::info_span!("store-attachments")),
    );
}

#[tracing::instrument(skip_all)]
async fn handle_quote(ctx: &client::Context, msg: &Message) -> Result<()> {
    let config = ctx.get_config().await;
    if msg.channel_id == config.channel_showcase {
        return Ok(());
    }

    lazy_static::lazy_static! {
        static ref MSG_LINK_PATTERN: Regex = Regex::new(r"<?https://(?:canary\.|ptb\.)?discord(?:app)?\.com/channels/(\d+)/(\d+)/(\d+)>?").unwrap();
    }
    let Some(caps) = MSG_LINK_PATTERN.captures(&msg.content) else { return Ok(()) };

    // Allow for explicitly escaping links by surrounding the quote in angle brackets
    let whole_match = caps.get(0).unwrap().as_str();
    if whole_match.starts_with('<') && whole_match.ends_with('>') {
        return Ok(());
    }

    debug!(quote.message_link = %whole_match, "Finished regex checking message for message link");

    let (guild_id, channel_id, message_id) = (
        caps.get(1).unwrap().as_str().parse::<GuildId>()?,
        caps.get(2).unwrap().as_str().parse::<ChannelId>()?,
        caps.get(3).unwrap().as_str().parse::<MessageId>()?,
    );

    let channel =
        channel_id.to_channel(&ctx).await?.guild().context("Message not in a guild-channel")?;
    let user_can_see_channel = channel
        .guild_id
        .member(&ctx, msg.author.id)
        .await?
        .permissions(ctx)?
        .read_message_history();

    debug!(quote.user_can_see_channel = ?user_can_see_channel, "checked if user can see the channel");

    if Some(guild_id) != msg.guild_id || !user_can_see_channel {
        return Ok(());
    }

    let mentioned_msg = ctx.http.get_message(channel_id, message_id).await?;
    let image_attachment = mentioned_msg
        .attachments
        .iter()
        .filter(|x| !x.filename.starts_with("SPOILER_"))
        .find(|x| x.dimensions().is_some());

    debug!("retrieved the mentioned message");

    if (image_attachment.is_none() && mentioned_msg.content.trim().is_empty())
        || mentioned_msg.author.bot
    {
        return Ok(());
    }

    msg.reply_embed(&ctx, |mut e| {
        if let Some(attachment) = image_attachment {
            e = e.image(&attachment.url);
        }
        e.footer(
            CreateEmbedFooter::new(format!("Quote of {}", mentioned_msg.author.tag()))
                .icon_url(mentioned_msg.author.face()),
        )
        .description(&mentioned_msg.content)
        .timestamp(mentioned_msg.timestamp)
    })
    .await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle_spam_protect(ctx: &client::Context, msg: &Message) -> Result<bool> {
    let account_age = Utc::now() - *msg.author.created_at();

    if msg.mentions.is_empty() || account_age > chrono::Duration::hours(24) {
        return Ok(false);
    }

    // TODO should the messages be cached here? given the previous checks this is rare enough to probably not matter.
    let msgs =
        msg.channel_id.messages(&ctx, GetMessages::default().before(msg.id).limit(10)).await?;

    let spam_msgs =
        msgs.iter().filter(|x| x.author == msg.author && x.content == msg.content).collect_vec();

    let is_spam = spam_msgs.len() > 3
        && match spam_msgs.iter().minmax_by_key(|x| *x.timestamp) {
            itertools::MinMaxResult::NoElements => false,
            itertools::MinMaxResult::OneElement(_) => false,
            itertools::MinMaxResult::MinMax(min, max) => {
                (*max.timestamp - *min.timestamp).num_minutes() < 2
            }
        };

    let default_pfp = msg.author.avatar.is_none();
    let ping_spam_msgs = msgs.iter().filter(|x| x.author == msg.author).collect_vec();
    let is_ping_spam = default_pfp
        && ping_spam_msgs.len() > 3
        && ping_spam_msgs.iter().map(|m| m.mentions.len()).sum::<usize>() as f32
            >= ping_spam_msgs.len() as f32 * 1.5
        && match spam_msgs.iter().minmax_by_key(|x| *x.timestamp) {
            itertools::MinMaxResult::NoElements => false,
            itertools::MinMaxResult::OneElement(_) => false,
            itertools::MinMaxResult::MinMax(min, max) => {
                (*max.timestamp - *min.timestamp).num_minutes() < 2
            }
        };

    if is_spam || is_ping_spam {
        let guild = msg.guild(&ctx.cache).context("Failed to load guild")?.to_owned();
        let member = guild.member(&ctx, msg.author.id).await?;
        let bot_id = ctx.cache.current_user().id;

        let duration = std::time::Duration::from_secs(60 * 30);

        commands::mute::apply_mute(
            &ctx,
            bot_id,
            member.into_owned(),
            duration,
            Some(format!("[AUTO] spamming \"{}\"", msg.content)),
            msg.link(),
        )
        .await?;
        modlog::log_mute_for_spamming(ctx, msg, duration).await;
        log_error!(
            msg.channel_id
                .delete_messages(&ctx, msgs.iter().filter(|m| m.author == msg.author))
                .await
        );
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tracing::instrument(skip_all)]
async fn handle_showcase_post(ctx: &client::Context, msg: &Message) -> Result<()> {
    if msg.kind == MessageType::ThreadCreated {
        tracing::debug!(msg = ?msg, "Deleting ThreadCreated message");
        msg.delete(&ctx).await.context("Failed to delete showcase ThreadCreated message")?;
    } else if msg.attachments.is_empty() && msg.embeds.is_empty() && !msg.content.contains("http") {
        tracing::debug!(msg = ?msg, "Deleting invalid showcase post");
        msg.delete(&ctx).await.context("Failed to delete invalid showcase submission")?;
        msg.author.direct_message(&ctx, CreateMessage::default()
                .content(indoc::indoc!("
                    Your showcase submission was detected to be invalid. If you wanna comment on a rice, create a thread.
                    If this is a mistake, contact the moderators or open an issue on https://github.com/unixporn/robbb
                "))
            ).await.context("Failed to send DM about invalid showcase submission")?;
    } else {
        msg.react(&ctx, ReactionType::Unicode("❤️".to_string()))
            .await
            .context("Error reacting to showcase submission with ❤️")?;

        if let Some(attachment) = msg.attachments.first() {
            if util::is_image_file(&attachment.filename) {
                let db = ctx.get_db().await;
                let fake_cdn_id = cdn_hack::FakeCdnId::from_message(msg, 0);
                db.update_fetch(
                    msg.author.id,
                    hashmap! { FetchField::Image => fake_cdn_id.encode() },
                )
                .await?;
            }
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle_feedback_post(ctx: &client::Context, msg: &Message) -> Result<()> {
    msg.react(&ctx, ReactionType::Unicode("👍".to_string()))
        .await
        .context("Error reacting to feedback submission with 👍")?;
    msg.react(&ctx, ReactionType::Unicode("👎".to_string()))
        .await
        .context("Error reacting to feedback submission with 👎")?;

    let thread_title = util::thread_title_from_text(&msg.content);
    if let Ok(title) = thread_title {
        msg.create_thread(&ctx, title)
            .await
            .context("Failed to create thread for feedback post")?;
    }

    // retrieve the last keep-at-bottom message the bot wrote
    let recent_messages = msg.channel_id.messages(&ctx, GetMessages::default().before(msg)).await?;

    let last_bottom_pin_msg = recent_messages.iter().find(|m| {
        m.author.bot && m.embeds.iter().any(|e| e.title == Some("CONTRIBUTING.md".to_string()))
    });
    if let Some(bottom_pin_msg) = last_bottom_pin_msg {
        bottom_pin_msg.delete(&ctx).await?;
    }

    let description = indoc::indoc!(
        "Before posting, please make sure to check if your idea is a **repetitive topic**. (Listed in pins)
        Note that we have added a consequence for failure. The inability to delete repetitive feedback will result in an 'unsatisfactory' mark on your official testing record, followed by death. Good luck!"
    );
    msg.channel_id
        .send_message(
            &ctx,
            CreateEmbed::default()
                .title("CONTRIBUTING.md")
                .color(0xb8bb26)
                .description(description)
                .into_create_message(),
        )
        .await?;
    Ok(())
}
