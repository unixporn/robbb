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

    if channel == config.channel_tech_support {
        Ok(send_ask_in_tech_support(ctx, channel, mentions, msg).await?)
    } else {
        Ok(send_move(ctx, channel, mentions, msg).await?)
    }
}

async fn send_ask_in_tech_support(
    ctx: &client::Context,
    channel: ChannelId,
    mentions: String,
    msg: &Message,
) -> CommandResult {
    let police_emote = ctx
        .get_up_emotes()
        .await
        .map(|emotes| emotes.police.to_string())
        .unwrap_or_default();

    msg.channel_id
        .send_embed(&ctx, |e| {
            e.author(|a| a.name(format!("Moved by {}", msg.author.tag())));
            e.description(indoc::formatdoc!(
                "{police}{police}**Please {} use `!ask <question>` to ask your question in {}**{police}{police}",
                mentions,
                channel.mention(),
                police = police_emote,
            ));
        })
        .await?;
    msg.delete(&ctx).await?;
    Ok(())
}

async fn send_move(
    ctx: &client::Context,
    channel: ChannelId,
    mentions: String,
    msg: &Message,
) -> CommandResult {
    let continuation_embed = make_create_embed(&ctx, |e| {
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
        .send_message(&ctx, |m| m.content(mentions).set_embed(continuation_embed))
        .await?;
    continuation_msg.guild_id = msg.guild_id;
    let police_emote = ctx
        .get_up_emotes()
        .await
        .map(|emotes| emotes.police.to_string())
        .unwrap_or_default();

    msg.channel_id
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
