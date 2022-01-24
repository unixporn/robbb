use std::collections::HashSet;

use crate::attachment_logging;
use crate::commands::fetch::FetchField;
use crate::log_error;
use chrono::Utc;
use itertools::Itertools;
use maplit::hashmap;
use regex::Regex;
use serenity::framework::Framework;
use tracing::debug;
use tracing_futures::Instrument;

use super::*;

pub async fn message_create(ctx: client::Context, msg: Message) -> Result<()> {
    let config = ctx.get_config().await;

    if msg.author.bot {
        return Ok(());
    }

    handle_attachment_logging(&ctx, &msg).await;

    if msg.channel_id == config.channel_showcase {
        log_error!(handle_showcase_post(&ctx, &msg).await);
    } else if msg.channel_id == config.channel_feedback {
        log_error!(handle_feedback_post(&ctx, &msg).await);
    } else if msg.channel_id == config.channel_tech_support {
        log_error!(handle_techsupport_post(ctx.clone(), &msg).await);
    }

    if msg.channel_id != config.channel_bot_messages && !msg.content.starts_with("!emojistats") {
        match handle_msg_emoji_logging(&ctx, &msg).await {
            Ok(emoji_used) => {
                tracing::Span::current().record("message_create.emoji_used", &emoji_used);
            }
            err => log_error!("Error while handling emoji logging", err),
        }
    }

    match handle_spam_protect(&ctx, &msg).await {
        Ok(stop) => {
            tracing::Span::current().record("message_create.stopped_at_spam_protect", &stop);
            if stop {
                return Ok(());
            }
        }
        err => log_error!("error while handling spam-protection", err),
    };
    match handle_blocklist::handle_blocklist(&ctx, &msg).await {
        Ok(stop) => {
            tracing::Span::current().record("message_create.stopped_at_blocklist", &stop);
            if stop {
                return Ok(());
            }
        }
        err => log_error!("error while handling blocklist", err),
    };

    match handle_highlighting(&ctx, &msg).await {
        Ok(notified_users) => {
            tracing::Span::current().record("message_create.notified_user_cnt", &notified_users);
        }
        err => log_error!("error while checking/handling highlights", err),
    }

    match handle_quote(&ctx, &msg).await {
        Ok(stop) => {
            tracing::Span::current().record("message_create.stopped_at_quote", &stop);
            if stop {
                return Ok(());
            }
        }
        err => log_error!("error while Handling a quoted message", err),
    };

    if msg.channel_id != config.channel_showcase || msg.is_private() {
        let framework = ctx
            .data
            .read()
            .await
            .get::<crate::FrameworkKey>()
            .unwrap()
            .clone();
        framework.dispatch(ctx, msg).await;
    }
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn handle_techsupport_post(ctx: client::Context, msg: &Message) -> Result<()> {
    let config = ctx.get_config().await;
    if msg.content.starts_with("!ask") {
        return Ok(());
    }

    let result = msg.author.dm(&ctx, |m| {
        m.content(format!(
            "Your message in {} has been deleted. Please use `!ask` to ask any questions, and respond in the thread.\nYour messages was:\n\n{}", 
            config.channel_tech_support.mention(),
            msg.content,
        ))
    }).await;

    if result.is_ok() {
        msg.delete(&ctx).await?;
    } else {
        let error_msg = msg.reply_error(&ctx, "Please use !ask to ask any questions and respond to others in the thread.\n**Your message will be deleted in a few seconds.**").await?;
        tokio::spawn({
            let ctx = ctx.clone();
            let msg = msg.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                log_error!(msg.delete(&ctx).await);
                log_error!(error_msg.delete(ctx).await);
            }
            .instrument(tracing::info_span!("techsupport-delete-message"))
        });
    }
    Ok(())
}

#[tracing::instrument(skip_all, fields(highlights.notified_user_cnt))]
async fn handle_highlighting(ctx: &client::Context, msg: &Message) -> Result<usize> {
    // don't trigger on bot commands
    if msg.content.starts_with('!') {
        tracing::Span::current().record("highlights.notified_user_cnt", &0);
        return Ok(0);
    }

    let (config, db) = ctx.get_config_and_db().await;

    let highlights_data = db.get_highlights().await?;

    let highlight_matches = tokio::task::spawn_blocking({
        let msg_content = msg.content.to_string();
        move || highlights_data.get_triggers_for_message(&msg_content)
    })
    .instrument(tracing::debug_span!("highlights-trigger-check"))
    .await?;

    if highlight_matches.is_empty() {
        tracing::Span::current().record("highlights.notified_user_cnt", &0);
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
        || config.category_mod_private == channel.category_id.context("Couldn't get category_id")?
    {
        return Ok(0);
    }

    let mut handled_users = HashSet::new();
    for (word, users) in highlight_matches {
        let mut embed = serenity::builder::CreateEmbed::default();
        embed
            .title("Highlight notification")
            .description(indoc::formatdoc!(
                "`{}` has been mentioned in {}
                [link to message]({})

                Don't care about this anymore?
                Run `!highlights remove {}` in #bot to stop getting these notifications.",
                word,
                msg.channel_id.mention(),
                msg.link(),
                word
            ))
            .author(|a| {
                a.name(&msg.author.tag());
                a.icon_url(&msg.author.face())
            })
            .timestamp(&msg.timestamp)
            .footer(|f| f.text(format!("#{}", channel.name)));

        tracing::debug!(
            highlights.word = %word,
            highlights.users = ?users,
            highlights.users_count = %users.len(),
            "Notifying {} users about a mention of the word '{}'", users.len(), word
        );
        for user_id in users {
            if user_id == msg.author.id
                // check if the user has already been notified of another word in this message
                || handled_users.contains(&user_id)
                // check if the user can read that channel
                || !channel.permissions_for_user(&ctx, user_id).await?.read_messages()
            {
                continue;
            }
            handled_users.insert(user_id);

            if let Ok(dm_channel) = user_id.create_dm_channel(&ctx).await {
                let _ = dm_channel
                    .send_message(&ctx, |m| m.set_embed(embed.clone()))
                    .await;
            }
        }
    }

    tracing::Span::current().record("highlights.notified_user_cnt", &handled_users.len());
    Ok(handled_users.len())
}

#[tracing::instrument(skip_all, fields(msg_emoji_logging.emoji_used))]
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
            &EmojiIdentifier {
                name: emoji.name.clone(),
                id: emoji.id,
                animated: emoji.animated,
            },
        )
        .await?;
    }
    tracing::Span::current().record("msg_emoji_logging.emoji_used", &actual_emojis.len());
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
async fn handle_quote(ctx: &client::Context, msg: &Message) -> Result<bool> {
    lazy_static::lazy_static! {
        static ref MSG_LINK_PATTERN: Regex = Regex::new(r#"<?https://(?:canary|ptb\.)?discord(?:app)?\.com/channels/(\d+)/(\d+)/(\d+)>?"#).unwrap();
    }
    if msg.content.starts_with('!') {
        return Ok(false);
    }

    let caps = match MSG_LINK_PATTERN.captures(&msg.content) {
        Some(caps) => {
            let whole_match = caps.get(0).unwrap().as_str();
            if whole_match.starts_with('<') && whole_match.ends_with('>') {
                return Ok(false);
            } else {
                caps
            }
        }
        None => return Ok(false),
    };
    debug!("Finished regex checking message for message link");

    let (guild_id, channel_id, message_id) = (
        caps.get(1).unwrap().as_str().parse::<u64>()?,
        caps.get(2).unwrap().as_str().parse::<u64>()?,
        caps.get(3).unwrap().as_str().parse::<u64>()?,
    );

    let channel = ChannelId(channel_id)
        .to_channel(&ctx)
        .await?
        .guild()
        .context("Message not in a guild-channel")?;

    let user_can_see_channel = channel
        .permissions_for_user(&ctx, msg.author.id)
        .await?
        .read_messages();

    debug!("checked if user can see the channel");

    if Some(GuildId(guild_id)) != msg.guild_id || !user_can_see_channel {
        return Ok(false);
    }

    let mentioned_msg = ctx.http.get_message(channel_id, message_id).await?;
    let image_attachment = mentioned_msg
        .attachments
        .iter()
        .find(|x| x.dimensions().is_some());

    debug!("retrieved the mentioned message");

    if (image_attachment.is_none() && mentioned_msg.content.trim().is_empty())
        || mentioned_msg.author.bot
    {
        return Ok(false);
    }

    msg.reply_embed(&ctx, |e| {
        e.footer(|f| {
            f.text(format!("Quote of {}", mentioned_msg.author.tag()));
            f.icon_url(mentioned_msg.author.face())
        });
        if let Some(attachment) = image_attachment {
            e.image(&attachment.url);
        }
        e.description(&mentioned_msg.content);
        e.timestamp(&mentioned_msg.timestamp);
    })
    .await?;
    Ok(true)
}

#[tracing::instrument(skip_all)]
async fn handle_spam_protect(ctx: &client::Context, msg: &Message) -> Result<bool> {
    let account_age = Utc::now() - msg.author.created_at();

    if msg.mentions.is_empty() || account_age > chrono::Duration::hours(24) {
        return Ok(false);
    }

    // TODO should the messages be cached here? given the previous checks this is rare enough to probably not matter.
    let msgs = msg
        .channel_id
        .messages(&ctx, |m| m.before(msg.id).limit(10))
        .await?;

    let spam_msgs = msgs
        .iter()
        .filter(|x| x.author == msg.author && x.content == msg.content)
        .collect_vec();

    let is_spam = spam_msgs.len() > 3
        && match spam_msgs.iter().minmax_by_key(|x| x.timestamp) {
            itertools::MinMaxResult::NoElements => false,
            itertools::MinMaxResult::OneElement(_) => false,
            itertools::MinMaxResult::MinMax(min, max) => {
                (max.timestamp - min.timestamp).num_minutes() < 2
            }
        };

    let default_pfp = msg.author.avatar.is_none();
    let ping_spam_msgs = msgs.iter().filter(|x| x.author == msg.author).collect_vec();
    let is_ping_spam = default_pfp
        && ping_spam_msgs.len() > 3
        && ping_spam_msgs
            .iter()
            .map(|m| m.mentions.len())
            .sum::<usize>() as f32
            >= ping_spam_msgs.len() as f32 * 1.5
        && match spam_msgs.iter().minmax_by_key(|x| x.timestamp) {
            itertools::MinMaxResult::NoElements => false,
            itertools::MinMaxResult::OneElement(_) => false,
            itertools::MinMaxResult::MinMax(min, max) => {
                (max.timestamp - min.timestamp).num_minutes() < 2
            }
        };

    if is_spam || is_ping_spam {
        let config = ctx.get_config().await;

        let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
        let member = guild.member(&ctx, msg.author.id).await?;
        let bot_id = ctx.cache.current_user_id().await;

        let duration = std::time::Duration::from_secs(60 * 30);

        crate::commands::mute::do_mute(&ctx, guild, bot_id, member, duration, Some("spam")).await?;
        config
            .log_bot_action(&ctx, |e| {
                e.description(format!(
                    "User {} was muted for spamming\n{}",
                    msg.author.id.mention(),
                    msg.to_context_link(),
                ));
                e.field(
                    "Duration",
                    humantime::Duration::from(duration).to_string(),
                    false,
                );
            })
            .await;
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
        msg.delete(&ctx)
            .await
            .context("Failed to delete showcase ThreadCreated message")?;
    } else if msg.attachments.is_empty() && msg.embeds.is_empty() && !msg.content.contains("http") {
        tracing::debug!(msg = ?msg, "Deleting invalid showcase post");
        msg.delete(&ctx)
            .await
            .context("Failed to delete invalid showcase submission")?;
        msg.author.direct_message(&ctx, |f| {
                f.content(indoc!("
                    Your showcase submission was detected to be invalid. If you wanna comment on a rice, create a thread.
                    If this is a mistake, contact the moderators or open an issue on https://github.com/unixporn/robbb
                "))
            }).await.context("Failed to send DM about invalid showcase submission")?;
    } else {
        msg.react(&ctx, ReactionType::Unicode("❤️".to_string()))
            .await
            .context("Error reacting to showcase submission with ❤️")?;

        if let Some(attachment) = msg.attachments.first() {
            if crate::util::is_image_file(&attachment.filename) {
                let db = ctx.get_db().await;
                db.update_fetch(
                    msg.author.id,
                    hashmap! { FetchField::Image => attachment.url.to_string() },
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
    let recent_messages = msg.channel_id.messages(&ctx, |m| m.before(msg)).await?;

    let last_bottom_pin_msg = recent_messages.iter().find(|m| {
        m.author.bot
            && m.embeds
                .iter()
                .any(|e| e.title == Some("CONTRIBUTING.md".to_string()))
    });
    if let Some(bottom_pin_msg) = last_bottom_pin_msg {
        bottom_pin_msg.delete(&ctx).await?;
    }
    msg.channel_id.send_message(&ctx, |m| {
        m.embed(|e| {
            e.title("CONTRIBUTING.md").color(0xb8bb26);
            e.description(indoc::indoc!(
                "Before posting, please make sure to check if your idea is a **repetitive topic**. (Listed in pins)
                Note that we have added a consequence for failure. The inability to delete repetitive feedback will result in an 'unsatisfactory' mark on your official testing record, followed by death. Good luck!"
            ))
        })
    }).await?;
    Ok(())
}
