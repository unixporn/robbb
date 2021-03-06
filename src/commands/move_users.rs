use super::*;
/// Move a conversation to a different channel.
#[command("move")]
#[usage("move <#channel> [<user> ...]")]
pub async fn move_users(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel = args
        .single::<ChannelId>()
        .invalid_usage(&MOVE_USERS_COMMAND_OPTIONS)?;
    let rest = args.remains().unwrap_or_default();
    let continuation_msg = channel
        .send_message(&ctx, |m| {
            m.content(format!(
                "{} {}\nContinuation from {}\n(<{}>)",
                msg.author.mention(),
                rest,
                msg.channel_id.mention(),
                msg.link()
            ))
        })
        .await?;
    let _ = msg
        .reply_embed(&ctx, |e| {
            e.description(format!(
                "Continued at {}\n{}",
                channel.mention(),
                continuation_msg.link()
            ));
        })
        .await?;
    Ok(())
}
