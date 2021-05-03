use std::collections::HashSet;

use crate::checks::{self, PermissionLevel};
use crate::log_error;
use crate::{attachment_logging, db::note::NoteType};
use chrono::Utc;
use itertools::Itertools;
use maplit::hashmap;
use regex::Regex;
use serenity::framework::Framework;

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
    }

    handle_emoji_logging(&ctx, &msg).await?;

    match handle_spam_protect(&ctx, &msg).await {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        err => log_error!("error while handling spam-protection", err),
    };
    match handle_blocklist(&ctx, &msg).await {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        err => log_error!("error while handling blocklist", err),
    };

    match handle_highlighting(&ctx, &msg).await {
        Ok(_) => {}
        err => log_error!("error while checking/handling highlights", err),
    }

    match handle_quote(&ctx, &msg).await {
        Ok(true) => return Ok(()),
        Ok(false) => {}
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

async fn handle_highlighting(ctx: &client::Context, msg: &Message) -> Result<()> {
    // don't trigger on bot commands
    if msg.content.starts_with('!') {
        return Ok(());
    }

    let (config, db) = ctx.get_config_and_db().await;

    let channel = msg
        .channel(&ctx)
        .await
        .context("Couldn't get channel")?
        .guild()
        .context("Couldn't get server")?;

    if config.category_mod_private == channel.category_id.context("Couldn't get category_id")? {
        return Ok(());
    }

    let highlights_data = db.get_highlights().await?;

    let mut handled_users = HashSet::new();

    for (word, users) in highlights_data.get_triggers_for_message(&msg.content) {
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

        for user_id in users {
            if user_id == msg.author.id || handled_users.contains(&user_id) {
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
    Ok(())
}

async fn handle_emoji_logging(ctx: &client::Context, msg: &Message) -> Result<()> {
    let guild_emojis = ctx.http.get_emojis(msg.guild_id.unwrap().0).await?;
    let actual_emojis = util::find_emojis(&msg.content)
        .iter()
        .filter_map(|iden| guild_emojis.iter().find(|a| a.id == iden.id))
        .dedup_by_with_count(|x, y| x.id == y.id)
        .collect_vec();
    if actual_emojis.is_empty() {
        return Ok(());
    }
    let data = ctx.data.read().await;
    let db = data.get::<Db>().unwrap();
    for (count, emoji) in actual_emojis {
        db.increment_emoji_text(
            count as u64,
            &EmojiIdentifier {
                name: emoji.name.clone(),
                id: emoji.id,
                animated: emoji.animated,
            },
        )
        .await?;
    }
    Ok(())
}

async fn handle_attachment_logging(ctx: &client::Context, msg: &Message) {
    if msg.attachments.is_empty() {
        return;
    }
    let config = ctx.get_config().await;

    let msg_id = msg.id;
    let channel_id = msg.channel_id;

    let attachments = msg.attachments.clone();
    tokio::spawn(async move {
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
    });
}

async fn handle_quote(ctx: &client::Context, msg: &Message) -> Result<bool> {
    lazy_static::lazy_static! {
        static ref MSG_LINK_PATTERN: Regex = Regex::new(r#"https://(?:canary\.)?discord(?:app)?\.com/channels/(\d+)/(\d+)/(\d+)"#).unwrap();
    }

    let caps = match MSG_LINK_PATTERN.captures(&msg.content) {
        Some(caps) => caps,
        None => return Ok(false),
    };

    let (guild_id, channel_id, message_id) = (
        caps.get(1).unwrap().as_str().parse::<u64>()?,
        caps.get(2).unwrap().as_str().parse::<u64>()?,
        caps.get(3).unwrap().as_str().parse::<u64>()?,
    );

    if Some(GuildId(guild_id)) != msg.guild_id {
        return Ok(false);
    }

    let mentioned_msg = ctx.http.get_message(channel_id, message_id).await?;
    let image_attachment = mentioned_msg
        .attachments
        .iter()
        .find(|x| x.dimensions().is_some());

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

async fn handle_blocklist(ctx: &client::Context, msg: &Message) -> Result<bool> {
    // don't block words by moderators
    let permission_level = checks::get_permission_level(&ctx, &msg).await;
    if permission_level == PermissionLevel::Mod {
        return Ok(false);
    }

    let (config, db) = ctx.get_config_and_db().await;

    let blocklist_regex = db.get_combined_blocklist_regex().await?;
    if let Some(word) = blocklist_regex.find(&msg.content) {
        let word = word.as_str();

        let dm_future = async {
            let _ = msg
                .author
                .dm(&ctx, |m| {
                    m.embed(|e| {
                        e.description(&msg.content).title(format!(
                            "Your message has been deleted for containing a blocked word: `{}`",
                            word
                        ))
                    })
                })
                .await;
        };

        let bot_log_future = config.log_automod_action(&ctx, |e| {
            e.author(|a| a.name("Message Autodelete"));
            e.title(format!(
                "{} - deleted because of `{}`",
                msg.author.tag(),
                word,
            ));
            e.description(format!("{} {}", msg.content, msg.to_context_link()));
        });

        let note_future = async {
            let bot_id = ctx.cache.current_user_id().await;
            let note_content = format!("Message deleted because of word `{}`", word);
            let _ = db
                .add_note(
                    bot_id,
                    msg.author.id,
                    note_content,
                    Utc::now(),
                    NoteType::BlocklistViolation,
                )
                .await;
        };

        tokio::join!(dm_future, bot_log_future, note_future, msg.delete(&ctx)).3?;

        Ok(true)
    } else {
        Ok(false)
    }
}

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

async fn handle_showcase_post(ctx: &client::Context, msg: &Message) -> Result<()> {
    if msg.attachments.is_empty() && msg.embeds.is_empty() && !msg.content.contains("http") {
        msg.delete(&ctx)
            .await
            .context("Failed to delete invalid showcase submission")?;
        msg.author.direct_message(&ctx, |f| {
                f.content(indoc!("
                    Your showcase submission was detected to be invalid. If you wanna comment on a rice, use the #ricing-theming channel.
                    If this is a mistake, contact the moderators or open an issue on https://github.com/unixporn/trup-rs
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
                    hashmap! { crate::commands::fetch::IMAGE_KEY.to_string() => attachment.url.to_string() },
                ).await?;
            }
        }
    }
    Ok(())
}

async fn handle_feedback_post(ctx: &client::Context, msg: &Message) -> Result<()> {
    msg.react(&ctx, ReactionType::Unicode("👍".to_string()))
        .await
        .context("Error reacting to feedback submission with 👍")?;
    msg.react(&ctx, ReactionType::Unicode("👎".to_string()))
        .await
        .context("Error reacting to feedback submission with 👎")?;

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
