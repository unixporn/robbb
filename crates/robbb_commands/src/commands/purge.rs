use chrono::Utc;

use super::*;

/// Delete recent messages of a user. Cannot delete messages older than 14 days.
#[poise::command(
    slash_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator"
)]
pub async fn purge(
    ctx: Ctx<'_>,
    #[description = "User id of the bad guy"] user: UserId,
    #[description = "How far back should we delete?"] duration: Option<humantime::Duration>,
    #[min = 1]
    #[max = 100]
    #[description = "How many messages should we delete?"]
    count: Option<usize>,
) -> Res<()> {
    let channel = ctx.guild_channel().await?;
    // 100 is the maximal amount of messages that we can fetch at all
    let count = count.unwrap_or(100);
    // discord does not let us bulk-delete messages older than 14 days
    let too_old_timestamp = Utc::now().timestamp() - 60 * 60 * 24 * 14;

    let response_msg = ctx.say("Purging their messages").await?.message().await?;

    let recent_messages = channel
        .messages(&ctx.discord(), |m| m.limit(100).before(response_msg.id))
        .await?
        .into_iter()
        .filter(|msg| msg.author.id == user)
        .take_while(|msg| {
            let msg_timestamp = msg.timestamp.timestamp();
            msg_timestamp > too_old_timestamp
                && duration
                    .map(|d| msg_timestamp > Utc::now().timestamp() - (d.as_secs() as i64))
                    .unwrap_or(true)
        })
        .take(count)
        .collect_vec();

    channel
        .delete_messages(&ctx.discord(), &recent_messages)
        .await?;
    ctx.say_success(format!(
        "Successfully deleted {} messages",
        recent_messages.len()
    ))
    .await?;
    Ok(())
}
