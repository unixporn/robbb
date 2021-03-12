use crate::extensions::{build_embed_builder, ChannelIdExt};

use super::*;
/// Move a conversation to a different channel.
#[command("move")]
#[usage("move <#channel> [<user> ...]")]
pub async fn move_users(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let channel = args
        .single::<ChannelId>()
        .invalid_usage(&MOVE_USERS_COMMAND_OPTIONS)?;
    let mentions = args
        .iter::<UserId>()
        .filter_map(|x| Some(x.ok()?.mention()))
        .join(" ");

    let embed_builder = build_embed_builder(&ctx).await;
    let continuation_msg = channel
        .send_message(&ctx, |m| {
            m.content(format!("{} {}", msg.author.mention(), mentions,));
            m.embed(|e| {
                embed_builder(e);
                e.author(|a| a.name(format!("Moved by {}", msg.author.tag())));
                e.description(indoc::formatdoc!(
                    "Continuation from {}
                    [Conversation]({})",
                    msg.channel_id.mention(),
                    msg.link()
                ))
            })
        })
        .await?;

    let _ = msg
        .channel_id
        .send_embed(&ctx, |e| {
            e.author(|a| a.name(format!("Moved by {}", msg.author.tag())));
            e.description(indoc::formatdoc!(
                "Continued at {}: [Conversation]({})
                Please continue your conversation **there**!",
                channel.mention(),
                continuation_msg.link()
            ));
        })
        .await?;

    msg.delete(&ctx).await?;
    Ok(())
}
