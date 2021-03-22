use std::str::FromStr;

use super::*;

#[derive(Debug)]
enum DeletionRange {
    Amount(usize),
    Duration(std::time::Duration),
}

impl FromStr for DeletionRange {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<usize>().map(DeletionRange::Amount).or_else(|_| {
            humantime::Duration::from_str(s).map(|d| DeletionRange::Duration(d.into()))
        })?)
    }
}

/// **silently** delete <amount> messages sent by <user> in the current channel or messages sent in the last <duration> by <user>.
/// Doesn't delete messages older than 14 days.
#[command]
#[usage("spurge <amount OR duration> <@user>")]
#[help_available(false)]
pub async fn spurge(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    do_purge(&ctx, &msg, args, true).await
}

/// delete <amount> messages sent by <user> in the current channel or messages sent in the last <duration> by <user>.
/// Doesn't delete messages older than 14 days.
/// (use `spurge` to do this silently)
#[command]
#[usage("purge <amount OR duration> <@user>")]
pub async fn purge(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    do_purge(&ctx, &msg, args, false).await
}

async fn do_purge(
    ctx: &client::Context,
    msg: &Message,
    mut args: Args,
    silent: bool,
) -> CommandResult {
    let range = args
        .single::<DeletionRange>()
        .invalid_usage(&PURGE_COMMAND_OPTIONS)?;

    let mentioned_user_id = args
        .single::<UserId>()
        .invalid_usage(&PURGE_COMMAND_OPTIONS)?;

    let channel = msg
        .channel(&ctx)
        .await
        .context("Failed to load channel")?
        .guild()
        .context("Failed to load GuildChannel")?;

    let msg_count = match range {
        DeletionRange::Amount(n) => n,
        _ => 100, // maximal amount of messages that we can fetch at all
    };

    // discord does not let us bulk-delete messages older than 14 days
    let too_old_timestamp = Utc::now().timestamp() - 60 * 60 * 24 * 14;

    let recent_messages = channel
        .messages(&ctx, |m| m.limit(100).before(msg.id))
        .await?
        .into_iter()
        .filter(|msg| msg.author.id == mentioned_user_id)
        .take_while(|msg| {
            msg.timestamp.timestamp() > too_old_timestamp
                && match range {
                    DeletionRange::Duration(d) => {
                        msg.timestamp.timestamp() > Utc::now().timestamp() - (d.as_secs() as i64)
                    }
                    _ => true,
                }
        })
        .take(msg_count)
        .collect_vec();

    channel.delete_messages(&ctx, &recent_messages).await?;
    if silent {
        msg.delete(&ctx).await?;
    } else {
        msg.reply_success(
            &ctx,
            format!("Successfully deleted {} messages", recent_messages.len()),
        )
        .await?;
    }
    Ok(())
}
