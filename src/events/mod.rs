use std::sync::Arc;

use crate::{db::mute, extensions::*};
use crate::{log_error, UpEmotes};
use anyhow::{Context, Result};

use serenity::async_trait;
use serenity::model::prelude::*;
use serenity::prelude::*;

use serenity::client;

use crate::{db::Db, util, Config};
use indoc::indoc;

mod guild_member_addition;
mod guild_member_removal;
mod handle_blocklist;
mod message_create;
mod message_delete;
mod message_update;
mod reaction_add;
mod ready;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: client::Context, data_about_bot: Ready) {
        log_error!(
            "Error while handling ready event",
            ready::ready(ctx, data_about_bot).await
        )
    }

    async fn message(&self, ctx: client::Context, msg: Message) {
        log_error!(
            "Error while handling message event",
            message_create::message_create(ctx, msg).await
        )
    }

    async fn message_update(
        &self,
        ctx: client::Context,
        old_if_available: Option<Message>,
        _new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        log_error!(
            "Error while handling message_update event",
            message_update::message_update(ctx, old_if_available, _new, event).await
        );
    }

    async fn message_delete(
        &self,
        ctx: client::Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        log_error!(
            "Error while handling message_delete event",
            message_delete::message_delete(ctx, channel_id, deleted_message_id, guild_id).await
        );
    }

    async fn message_delete_bulk(
        &self,
        ctx: client::Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
        guild_id: Option<GuildId>,
    ) {
        log_error!(
            "Error while handling message_delete event",
            message_delete::message_delete_bulk(
                ctx,
                channel_id,
                multiple_deleted_messages_ids,
                guild_id,
            )
            .await
        );
    }

    async fn guild_member_addition(
        &self,
        ctx: client::Context,
        guild_id: GuildId,
        new_member: Member,
    ) {
        log_error!(
            "Error while handling guild_member_addition event",
            guild_member_addition::guild_member_addition(ctx, guild_id, new_member).await
        );
    }

    async fn guild_member_removal(
        &self,
        ctx: client::Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        log_error!(
            "Error while handling guild_member_removal event",
            guild_member_removal::guild_member_removal(ctx, guild_id, user, _member).await
        );
    }

    async fn reaction_add(&self, ctx: client::Context, event: Reaction) {
        log_error!(
            "Error while handling reaction_add event",
            reaction_add::reaction_add(ctx, event).await
        );
    }
}

async fn unmute(
    ctx: &client::Context,
    config: &Arc<Config>,
    db: &Arc<Db>,
    mute: &mute::Mute,
) -> Result<()> {
    db.set_mute_inactive(mute.id).await?;
    let mut member = config.guild.member(&ctx, mute.user).await?;
    member.remove_roles(&ctx, &[config.role_mute]).await?;

    Ok(())
}
