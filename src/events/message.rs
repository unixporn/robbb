use crate::log_error;
use crate::{attachment_logging, db::note::NoteType};
use anyhow::bail;
use chrono::Utc;
use itertools::Itertools;
use maplit::hashmap;
use regex::Regex;
use serenity::framework::Framework;

use super::*;
use reqwest::multipart;

pub async fn message(ctx: client::Context, msg: Message) -> Result<()> {
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
    match handle_quote(&ctx, &msg).await {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        err => log_error!("error while Handling a quoted message", err),
    };
    match handle_message_txt(&ctx, &msg).await {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        err => log_error!("error while handling a message.txt upload", err),
    };

    let framework = ctx
        .data
        .read()
        .await
        .get::<crate::FrameworkKey>()
        .unwrap()
        .clone();

    framework.dispatch(ctx, msg).await;

    Ok(())
}

async fn handle_attachment_logging(ctx: &client::Context, msg: &Message) {
    if msg.attachments.is_empty() {
        return;
    }
    let config = ctx.get_config().await;

    let msg_id = msg.id.clone();
    let channel_id = msg.channel_id.clone();

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

    let msgs = msgs
        .iter()
        .filter(|x| {
            x.author.id == msg.author.id
                && x.channel_id == msg.channel_id
                && x.content == msg.content
        })
        .collect_vec();

    let is_spam = msgs.len() > 3
        && match msgs.iter().minmax_by_key(|x| x.timestamp) {
            itertools::MinMaxResult::NoElements => true,
            itertools::MinMaxResult::OneElement(_) => true,
            itertools::MinMaxResult::MinMax(min, max) => {
                (max.timestamp - min.timestamp).num_minutes() < 2
            }
        };

    if is_spam {
        let config = ctx.get_config().await;

        let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
        let member = guild.member(&ctx, msg.author.id).await?;
        let bot_id = ctx.cache.current_user_id().await;

        let duration = std::time::Duration::from_secs(60 * 30);

        crate::commands::mute::do_mute(&ctx, guild, bot_id, member, duration.clone(), Some("spam"))
            .await?;
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
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn handle_showcase_post(ctx: &client::Context, msg: &Message) -> Result<()> {
    if msg.attachments.is_empty() && msg.embeds.is_empty() {
        msg.delete(&ctx)
            .await
            .context("Failed to delete invalid showcase submission")?;
        msg.author.direct_message(&ctx, |f| {
                f.content(indoc!("
                    Your showcase submission was detected to be invalid. If you wanna comment on a rice, use the #ricing-theming channel.
                    If this is a mistake, contact the moderators or open an issue on https://github.com/unixporn/trup
                "))
            }).await.context("Failed to send DM about invalid showcase submission")?;
    } else {
        if let Some(attachment) = msg.attachments.first() {
            let db = ctx.get_db().await;
            msg.react(&ctx, ReactionType::Unicode("❤️".to_string()))
                .await
                .context("Error reacting to showcase submission with ❤️")?;

            db.update_fetch(
                msg.author.id,
                hashmap! { crate::commands::fetch::IMAGE_KEY.to_string() => attachment.url.to_string() },
            )
            .await?;
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

async fn handle_message_txt(ctx: &client::Context, msg: &Message) -> Result<bool> {
    let message_txt_file = match msg.attachments.iter().find(|a| a.filename == "message.txt") {
        Some(attachment) => attachment,
        None => return Ok(false),
    };

    // Upload the file to 0x0.st via URL
    let form = multipart::Form::new().text("url", message_txt_file.url.clone());
    let code = reqwest::Client::builder()
        .build()?
        .post("https://0x0.st")
        .multipart(form)
        .send()
        .await?;
    if !code.status().is_success() {
        bail!(
            "0x0.st returned an error uploading the `message.txt` from {} ({}): \n{}",
            msg.author.name,
            msg.link(),
            code.text().await?
        );
    }

    let download_url = code.text().await?;
    let color = msg
        .guild(&ctx)
        .await
        .context("Failed to load guild")?
        .member(&ctx, msg.author.id)
        .await?
        .colour(&ctx)
        .await;

    msg.reply_embed(&ctx, |m| {
        m.title("Open message.txt in browser");
        m.url(download_url);
        m.color_opt(color);
        m.description(
            "Discord forces you to download the file, so here's an easier way to read that file.",
        );
    })
    .await?;
    Ok(true)
}
