use std::sync::Arc;

use crate::{db::mute, extensions::*};
use crate::{log_error, UserData};
use anyhow::{Context, Result};

use crate::prelude::Error;
use serenity::model::prelude::*;

use serenity::client;

use crate::{db::Db, util, Config};

//mod guild_member_addition;
//mod guild_member_removal;
mod guild_member_update;
//mod handle_blocklist;
//mod message_create;
//mod message_delete;
//mod message_update;
//mod reaction_add;
//mod reaction_remove;
pub mod ready;

pub async fn handle_event(
    ctx: &client::Context,
    event: &poise::Event<'_>,
    _framework: &poise::Framework<UserData, Error>,
    data: UserData,
) {
    use poise::Event::*;
    let result = match event {
        Ready { data_about_bot } => ready::ready(ctx.clone(), data, data_about_bot).await,
        GuildMemberUpdate {
            old_if_available,
            new,
        } => {
            guild_member_update::guild_member_update(
                ctx.clone(),
                old_if_available.clone(),
                new.clone(),
            )
            .await
        }
        _ => Ok(()),
    };

    log_error!(
        format!("Error while handling {} event", event.name(),),
        result
    );
}

pub struct Handler;

impl Handler {
    /*

    #[tracing::instrument(
        skip_all,
        fields(
            command_name, message_create.notified_user_cnt, message_create.stopped_at_spam_protect,
            message_create.stopped_at_blocklist, message_create.stopped_at_quote, message_create.emoji_used,
            %msg.content, msg.author = %msg.author.tag(), %msg.channel_id, %msg.id
            )
    )]
    pub async fn message(&self, ctx: &client::Context, msg: &Message) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling message event",
            message_create::message_create(ctx, msg).await
        )
    }

    #[tracing::instrument(skip_all, fields(msg.id = %event.id, msg.channel_id = %event.channel_id, ?event))]
    pub async fn message_update(
        &self,
        ctx: client::Context,
        old_if_available: Option<Message>,
        _new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling message_update event",
            message_update::message_update(ctx, old_if_available, _new, event).await
        );
    }

    #[tracing::instrument(skip_all, fields(msg.id = %deleted_message_id, msg.channel_id = %channel_id))]
    pub async fn message_delete(
        &self,
        ctx: client::Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling message_delete event",
            message_delete::message_delete(ctx, channel_id, deleted_message_id, guild_id).await
        );
    }

    #[tracing::instrument(skip_all)]
    pub async fn message_delete_bulk(
        &self,
        ctx: client::Context,
        channel_id: ChannelId,
        multiple_deleted_messages_ids: Vec<MessageId>,
        guild_id: Option<GuildId>,
    ) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
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

    #[tracing::instrument(skip_all, fields(member.tag = %new_member.user.tag()))]
    pub async fn guild_member_addition(&self, ctx: client::Context, new_member: Member) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling guild_member_addition event",
            guild_member_addition::guild_member_addition(ctx, new_member.guild_id, new_member)
                .await
        );
    }

    #[tracing::instrument(skip_all, fields(member.tag = %user.tag()))]
    pub async fn guild_member_removal(
        &self,
        ctx: client::Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling guild_member_removal event",
            guild_member_removal::guild_member_removal(ctx, guild_id, user, _member).await
        );
    }

    #[tracing::instrument(
        skip_all,
        fields(
            reaction.emoji = %event.emoji,
            reaction.channel_id = %event.channel_id,
            reaction.user_id = ?event.user_id,
            reaction.message_id = ?event.message_id
        )
    )]
    pub async fn reaction_add(&self, ctx: client::Context, event: Reaction) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling reaction_add event",
            reaction_add::reaction_add(ctx, event).await
        );
    }
    #[tracing::instrument(
        skip_all,
        fields(
            reaction.emoji = %event.emoji,
            reaction.channel_id = %event.channel_id,
            reaction.user_id = ?event.user_id,
            reaction.message_id = ?event.message_id
        )
    )]
    pub async fn reaction_remove(&self, ctx: client::Context, event: Reaction) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling reaction_remove event",
            reaction_remove::reaction_remove(ctx, event).await
        );
    }
    */
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
