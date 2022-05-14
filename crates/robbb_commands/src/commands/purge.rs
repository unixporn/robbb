use chrono::Utc;
use robbb_util::embeds;

use super::*;

/// the maximal amount of messages that we can fetch at all
const MAX_BULK_DELETE_CNT: usize = 100;
/// discord does not let us bulk-delete messages older than 14 days
const MAX_BULK_DELETE_AGO_SECS: i64 = 60 * 60 * 24 * 14;

/// Delete recent messages of a user. Cannot delete messages older than 14 days.
#[poise::command(
    slash_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator",
    default_member_permissions = "ADMINISTRATOR"
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
    let now_timestamp = Utc::now().timestamp();
    let count = count.unwrap_or(MAX_BULK_DELETE_CNT);
    let too_old_timestamp = now_timestamp - MAX_BULK_DELETE_AGO_SECS;

    let mut response_msg = ctx
        .send_embed(|e| {
            e.description("Purging their messages...");
        })
        .await?
        .message()
        .await?;

    let _working = ctx.defer_or_broadcast().await?;

    let recent_messages = channel
        .messages(&ctx.discord(), |m| m.limit(100).before(response_msg.id))
        .await?
        .into_iter()
        .filter(|msg| msg.author.id == user)
        .take_while(|msg| {
            let msg_timestamp = msg.timestamp.timestamp();
            msg_timestamp > too_old_timestamp
                && duration.map_or(true, |d| {
                    msg_timestamp > now_timestamp - (d.as_secs() as i64)
                })
        })
        .take(count)
        .collect_vec();

    channel
        .delete_messages(&ctx.discord(), &recent_messages)
        .await?;

    let success_embed = embeds::make_success_mod_action_embed(
        &ctx.discord(),
        &format!("Successfully deleted {} messages", recent_messages.len()),
    )
    .await;
    response_msg
        .edit(&ctx.discord(), |e| e.set_embed(success_embed))
        .await?;
    Ok(())
}
