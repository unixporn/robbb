use chrono::Utc;
use itertools::Itertools;
use maplit::hashmap;

use super::*;

pub async fn message(ctx: client::Context, msg: Message) -> Result<()> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    if msg.author.bot {
        return Ok(());
    }

    if handle_spam_protect(&ctx, &msg).await? {
        return Ok(());
    }

    if msg.channel_id == config.channel_showcase {
        handle_showcase_post(ctx, msg)
            .await
            .context("Failed to handle showcase post")
    } else if msg.channel_id == config.channel_feedback {
        handle_feedback_post(ctx, msg)
            .await
            .context("Failed to handle feedback post")
    } else {
        Ok(())
    }
}

async fn handle_spam_protect(ctx: &client::Context, msg: &Message) -> Result<bool> {
    let account_age_millis = Utc::now().timestamp() - msg.author.created_at().timestamp();

    if msg.mentions.is_empty() && (account_age_millis / 1000 / 60 / 60) > 24 {
        return Ok(false);
    }

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
        let data = ctx.data.read().await;
        let config = data.get::<Config>().unwrap().clone();

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

async fn handle_showcase_post(ctx: client::Context, msg: Message) -> Result<()> {
    if !msg.attachments.is_empty() || !msg.embeds.is_empty() {
        if let Some(attachment) = msg.attachments.first() {
            let data = ctx.data.read().await;
            let db = data.get::<Db>().unwrap().clone();
            msg.react(&ctx, ReactionType::Unicode("❤️".to_string()))
                .await
                .context("Error reacting to showcase submission with ❤️")?;

            db.update_fetch(
                msg.author.id,
                hashmap! {"image".to_string() => attachment.url.to_string() },
            )
            .await?;
        }
    } else {
        msg.delete(&ctx)
            .await
            .context("Failed to delete invalid showcase submission")?;
        msg.author.direct_message(&ctx, |f| {
                f.content(indoc!("
                    Your showcase submission was detected to be invalid. If you wanna comment on a rice, use the #ricing-theming channel.
                    If this is a mistake, contact the moderators or open an issue on https://github.com/unixporn/trup
                "))
            }).await.context("Failed to send DM about invalid showcase submission")?;
    }
    Ok(())
}

async fn handle_feedback_post(ctx: client::Context, msg: Message) -> Result<()> {
    msg.react(&ctx, ReactionType::Unicode("👍".to_string()))
        .await
        .context("Error reacting to feedback submission with 👍")?;
    msg.react(&ctx, ReactionType::Unicode("👎".to_string()))
        .await
        .context("Error reacting to feedback submission with 👍")?;

    // retrieve the last keep-at-bottom message the bot wrote
    let recent_messages = msg.channel_id.messages(&ctx, |m| m.before(&msg)).await?;

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
