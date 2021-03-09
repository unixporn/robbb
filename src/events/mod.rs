use crate::extensions::*;
use anyhow::{Context, Result};

use serenity::async_trait;
use serenity::model::prelude::*;
use serenity::prelude::*;

use serenity::client;

use crate::{db::Db, extensions::UserExt, util, Config};
use indoc::indoc;

mod guild_member_addition;
mod guild_member_removal;
mod message;
mod message_delete;
mod message_update;
mod reaction_add;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: client::Context, _data_about_bot: Ready) {
        println!("Trup is ready!");
        start_mute_handler(ctx).await;
    }

    async fn message(&self, ctx: client::Context, msg: Message) {
        util::log_error_value(
            message::message(ctx, msg)
                .await
                .context("Error while handling message event"),
        );
    }

    async fn message_update(
        &self,
        ctx: client::Context,
        old_if_available: Option<Message>,
        _new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        util::log_error_value(
            message_update::message_update(ctx, old_if_available, _new, event)
                .await
                .context("Error while handling message_update event"),
        );
    }

    async fn message_delete(
        &self,
        ctx: client::Context,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        util::log_error_value(
            message_delete::message_delete(ctx, channel_id, deleted_message_id, guild_id)
                .await
                .context("Error while handling message_delete event"),
        );
    }

    async fn guild_member_addition(
        &self,
        ctx: client::Context,
        guild_id: GuildId,
        new_member: Member,
    ) {
        util::log_error_value(
            guild_member_addition::guild_member_addition(ctx, guild_id, new_member)
                .await
                .context("Error while handling guild_member_addition event"),
        );
    }

    async fn guild_member_removal(
        &self,
        ctx: client::Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        util::log_error_value(
            guild_member_removal::guild_member_removal(ctx, guild_id, user, _member)
                .await
                .context("Error while handling guild_member_removal event"),
        );
    }

    async fn reaction_add(&self, ctx: client::Context, event: Reaction) {
        util::log_error_value(
            reaction_add::reaction_add(ctx, event)
                .await
                .context("Error while handling reaction_addd event"),
        )
    }
}

async fn start_mute_handler(ctx: client::Context) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let data = ctx.data.read().await;
            let config = data.get::<Config>().unwrap().clone();
            let db = data.get::<Db>().unwrap().clone();
            let mutes = match db.get_newly_expired_mutes().await {
                Ok(mutes) => mutes,
                Err(err) => {
                    eprintln!("Failed to request expired mutes: {}", err);
                    continue;
                }
            };
            for mute in mutes {
                let result: Result<_> = try {
                    let mut member = config.guild.member(&ctx, mute.user).await?;
                    member.remove_roles(&ctx, &[config.role_mute]).await?;
                    db.set_mute_inactive(mute.id).await?;
                    config
                        .log_bot_action(&ctx, |e| {
                            e.description(format!("{} is now unmuted", mute.user.mention()));
                        })
                        .await;
                };
                if let Err(err) = result {
                    eprintln!("Error handling mute removal: {}", err);
                }
            }
        }
    });
}
