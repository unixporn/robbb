use crate::embeds::make_create_embed;
use crate::extensions::ChannelIdExt;

use super::*;
/// Move a conversation to a different channel.
#[command("move")]
#[only_in(guilds)]
#[usage("move <#channel> [<user> ...]")]
pub async fn move_users(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let config = ctx.get_config().await;
    let channel = args
        .single::<ChannelId>()
        .invalid_usage(&MOVE_USERS_COMMAND_OPTIONS)?;

    if channel == msg.channel_id {
        abort_with!("You're already here!")
    } else if channel == config.channel_showcase || channel == config.channel_feedback {
        abort_with!("I won't move you there");
    }

    let mentions = args
        .iter::<UserId>()
        .filter_map(|x| Some(x.ok()?.mention()))
        .join(" ");

    let create_embed = make_create_embed(&ctx, |e| {
        e.author(|a| a.name(format!("Moved by {}", msg.author.tag())));
        e.description(indoc::formatdoc!(
            "Continuation from {}
                    [Conversation]({})",
            msg.channel_id.mention(),
            msg.link()
        ))
    })
    .await;

    let mut continuation_msg = channel
        .send_message(&ctx, |m| m.content(mentions).set_embed(create_embed))
        .await?;

    // WORKAROUND
    // Currently, Discords API seems to not set the guild_id field for message objects returned from send_message invocations.
    // tracking issue: https://github.com/serenity-rs/serenity/issues/832
    continuation_msg.guild_id = msg.guild_id;

    let police_emote = ctx
        .get_up_emotes()
        .await
        .map(|emotes| emotes.police.to_string())
        .unwrap_or_default();

    let _ = msg
        .channel_id
        .send_embed(&ctx, |e| {
            e.author(|a| a.name(format!("Moved by {}", msg.author.tag())));
            e.description(indoc::formatdoc!(
                "{police}{police}**MOVE THIS CONVERSATION!**{police}{police}
                Continued at {}: [Conversation]({})
                Please continue your conversation **there**!",
                channel.mention(),
                continuation_msg.link(),
                police = police_emote,
            ));
        })
        .await?;

    msg.delete(&ctx).await?;
    Ok(())
}
