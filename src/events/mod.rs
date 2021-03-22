use std::sync::Arc;

use crate::{db::mute, extensions::*};
use crate::{log_error, UPEmotes};
use anyhow::{Context, Result};

use serenity::async_trait;
use serenity::model::prelude::*;
use serenity::prelude::*;

use serenity::client;

use crate::{db::Db, util, Config};
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
        log::info!("Trup is ready!");
        {
            let config = ctx.get_config().await;

            match load_up_emotes(&ctx, config.guild).await {
                Ok(emotes) => {
                    ctx.data.write().await.insert::<UPEmotes>(Arc::new(emotes));
                }
                Err(err) => {
                    log::warn!("Error loading emotes: {}", err);
                }
            }
        }

        let _ = ctx
            .set_presence(
                Some(Activity::competing("being the very best")),
                OnlineStatus::Online,
            )
            .await;

        start_mute_handler(ctx.clone()).await;
        start_attachment_log_handler(ctx).await;
    }

    async fn message(&self, ctx: client::Context, msg: Message) {
        log_error!(
            "Error while handling message event",
            message::message(ctx, msg).await
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

async fn start_mute_handler(ctx: client::Context) {
    tokio::spawn(async move {
        let (config, db) = ctx.get_config_and_db().await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let mutes = match db.get_newly_expired_mutes().await {
                Ok(mutes) => mutes,
                Err(err) => {
                    log::error!("Failed to request expired mutes: {}", err);
                    continue;
                }
            };
            for mute in mutes {
                if let Err(err) = unmute(&ctx, &config, &db, &mute).await {
                    log::error!("Error handling mute removal: {}", err);
                } else {
                    config
                        .log_bot_action(&ctx, |e| {
                            e.description(format!("{} is now unmuted", mute.user.mention()));
                        })
                        .await;
                }
            }
        }
    });
}

async fn start_attachment_log_handler(ctx: client::Context) {
    tokio::spawn(async move {
        let config = ctx.get_config().await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            log_error!(
                "Failed to clean up attachments",
                crate::attachment_logging::cleanup(&config).await
            );
        }
    });
}

async fn load_up_emotes(ctx: &client::Context, guild: GuildId) -> Result<UPEmotes> {
    let all_emoji = guild.emojis(&ctx).await?;
    Ok(UPEmotes {
        pensibe: all_emoji
            .iter()
            .find(|x| x.name == "pensibe")
            .context("no pensibe emote found")?
            .clone(),
        police: all_emoji
            .iter()
            .find(|x| x.name == "police")
            .context("no police emote found")?
            .clone(),
        poggers: all_emoji
            .iter()
            .find(|x| x.name == "poggersphisch")
            .context("no police poggers found")?
            .clone(),
        stares: all_emoji
            .into_iter()
            .filter(|x| x.name.starts_with("stare"))
            .collect(),
    })
}
