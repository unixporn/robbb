use std::borrow::Cow;

use poise::serenity_prelude::{Channel, ChannelId, CreateEmbed, Message};

use robbb_util::{abort_with, embeds::make_create_embed};

use super::*;

/// Move a conversation to a different channel.
#[poise::command(slash_command, prefix_command, guild_only, rename = "move")]
pub async fn move_users(
    ctx: Ctx<'_>,
    #[description = "Channel to move to"] target_channel: Channel,
    #[description = "Users to move"]
    #[rest]
    users: Option<String>,
) -> Res<()> {
    let target_channel = target_channel.id();
    let config = ctx.get_config();
    let users = users.unwrap_or_default();

    if target_channel == ctx.channel_id() {
        abort_with!("You're already here!")
    } else if target_channel == config.channel_showcase || target_channel == config.channel_feedback
    {
        abort_with!("I won't move you there");
    }

    let mentions =
        users.split(' ').filter_map(|x| Some(x.trim().parse::<UserId>().ok()?.mention())).join(" ");

    if target_channel == config.channel_tech_support {
        Ok(send_ask_in_tech_support(ctx, target_channel, mentions).await?)
    } else {
        Ok(send_move(ctx, target_channel, mentions).await?)
    }
}

async fn send_ask_in_tech_support(
    ctx: Ctx<'_>,
    target_channel: ChannelId,
    mentions: String,
) -> Res<()> {
    let police_emote = ctx
        .data()
        .up_emotes
        .read()
        .as_ref()
        .map(|emotes| emotes.police.to_string())
        .unwrap_or_default();

    ctx.send_embed(|e| {
        e.author_user(ctx.author());
        e.description(indoc::formatdoc!(
            "{police}{police}**Please {} use `/ask` to ask your question in {}**{police}{police}",
            mentions,
            target_channel.mention(),
            police = police_emote,
        ));
    })
    .await?;
    Ok(())
}

async fn send_move(ctx: Ctx<'_>, target_channel: ChannelId, mentions: String) -> Res<()> {
    // we put this in a function so we can easily generate a version that contains the link back to the context
    // and one that doesn't yet.
    // Because slash commands aren't messages, we need to first send a message that we can then link to.
    // Because we want two-way links, we need to edit one of the messages to edit in the link later on.
    async fn make_continuation_embed<'a>(
        ctx: Ctx<'_>,
        continuation_msg: Option<Cow<'a, Message>>,
    ) -> CreateEmbed {
        make_create_embed(ctx.discord(), |e| {
            e.author_user(ctx.author());
            e.description(indoc::formatdoc!(
                "Continuation from {}
                    [Conversation]({})",
                ctx.channel_id().mention(),
                continuation_msg.map(|x| x.link()).unwrap_or_default(),
            ))
        })
        .await
    }

    let mut continuation_msg = {
        let continuation_embed = make_continuation_embed(ctx, None).await;
        target_channel
            .send_message(&ctx.discord(), |m| m.content(mentions).set_embed(continuation_embed))
            .await?
    };

    continuation_msg.guild_id = ctx.guild_id();
    let police_emote = ctx
        .data()
        .up_emotes
        .read()
        .as_ref()
        .map(|emotes| emotes.police.to_string())
        .unwrap_or_default();

    let move_message = ctx
        .send_embed(|e| {
            e.author_user(ctx.author());
            e.description(indoc::formatdoc!(
                "{police}{police}**MOVE THIS CONVERSATION!**{police}{police}
                Continued at {}: [Conversation]({})
                Please continue your conversation **there**!",
                target_channel.mention(),
                continuation_msg.link(),
                police = police_emote,
            ));
        })
        .await?;
    let move_message = move_message.message().await?;

    let new_continuation_embed = make_continuation_embed(ctx, Some(move_message)).await;
    continuation_msg.edit(&ctx.discord(), |m| m.set_embed(new_continuation_embed)).await?;

    if let poise::Context::Prefix(ctx) = ctx {
        ctx.msg.delete(&ctx.discord).await?;
    }
    Ok(())
}
