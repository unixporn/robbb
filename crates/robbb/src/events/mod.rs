use std::sync::Arc;

use anyhow::{Context, Result};

use poise::async_trait;
use poise::serenity_prelude::ShardManager;
use robbb_db::Db;
use robbb_util::{config::Config, log_error, prelude::Error, util, UserData};
use robbb_util::{extensions::*, UpEmotes};
use serenity::model::prelude::*;

use serenity::client;

mod guild_member_addition;
mod guild_member_removal;
mod guild_member_update;
mod handle_blocklist;
mod message_create;
mod message_delete;
mod message_update;
mod reaction_add;
mod reaction_remove;
pub mod ready;

pub struct Handler {
    pub options: poise::FrameworkOptions<UserData, Error>,
    pub shard_manager: parking_lot::RwLock<Option<Arc<tokio::sync::Mutex<ShardManager>>>>,
    pub bot_id: parking_lot::RwLock<Option<UserId>>,
    pub user_data: UserData,
}

impl Handler {
    pub fn new(options: poise::FrameworkOptions<UserData, Error>, user_data: UserData) -> Self {
        Self {
            options,
            user_data,
            shard_manager: parking_lot::RwLock::new(None),
            bot_id: parking_lot::RwLock::new(None),
        }
    }

    pub fn set_shard_manager(&self, manager: Arc<tokio::sync::Mutex<ShardManager>>) {
        *self.shard_manager.write() = Some(manager);
    }

    async fn init_ready_data(&self, ctx: &client::Context, user_id: UserId) {
        let _ = self.bot_id.write().insert(user_id);
        match robbb_util::load_up_emotes(&ctx, self.user_data.config.guild).await {
            Ok(up_emotes) => {
                let up_emotes = Arc::new(up_emotes);
                let _ = self.user_data.up_emotes.write().insert(up_emotes.clone());
                ctx.data.write().await.insert::<UpEmotes>(up_emotes);
            }
            Err(error) => tracing::error!(%error, "Failed to load up-emotes"),
        }
    }

    #[tracing::instrument(skip_all, fields(
        event.name = %event.name(),
        command_name,
        msg.content,
        msg.author,
        msg.id,
        msg.channel_id,
        msg.channel,
    ))]
    async fn dispatch_poise_event(&self, ctx: &client::Context, event: &poise::Event<'_>) {
        let shard_manager = (*self.shard_manager.read()).clone().unwrap();
        let framework_data = poise::FrameworkContext {
            bot_id: self.bot_id.read().unwrap_or_else(|| UserId(0)),
            options: &self.options,
            user_data: &self.user_data,
            shard_manager: &shard_manager,
        };
        poise::dispatch_event(framework_data, ctx, event).await;
    }
}

#[async_trait]
impl client::EventHandler for Handler {
    #[tracing::instrument(skip_all)]
    async fn ready(&self, ctx: client::Context, data_about_bot: Ready) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();

        self.init_ready_data(&ctx, data_about_bot.user.id).await;

        log_error!(
            "Error while handling ready event",
            ready::ready(ctx, data_about_bot).await
        );
    }

    #[tracing::instrument(
        skip_all,
        fields(
            command_name, message_create.notified_user_cnt, message_create.stopped_at_spam_protect,
            message_create.stopped_at_blocklist, message_create.stopped_at_quote, message_create.emoji_used,
            %msg.content, msg.author = %msg.author.tag(), %msg.channel_id, %msg.id
        )
    )]
    async fn message(&self, ctx: client::Context, msg: Message) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        let stop_event_handler =
            match message_create::message_create(ctx.clone(), msg.clone()).await {
                Ok(stop_event_handler) => stop_event_handler,
                Err(e) => {
                    tracing::error!(error.message = %format!("{}", &e), "{:?}", e);
                    false
                }
            };
        if !stop_event_handler {
            self.dispatch_poise_event(&ctx, &poise::Event::Message { new_message: msg })
                .await;
        }
    }

    #[tracing::instrument(
        skip_all,
        fields(
            interaction_create.kind = ?interaction.kind(),
        )
    )]
    async fn interaction_create(&self, ctx: client::Context, interaction: Interaction) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();

        let stop_event_handler =
            match handle_blocklist::handle_blocklist_in_interaction(&ctx, &interaction).await {
                Ok(stop_event_handler) => stop_event_handler,
                Err(e) => {
                    tracing::error!(error.message = %format!("{}", &e), "{:?}", e);
                    false
                }
            };

        log_error!(
            robbb_commands::commands::ask::handle_ask_button_clicked(&ctx, &interaction).await
        );

        if !stop_event_handler {
            self.dispatch_poise_event(&ctx, &poise::Event::InteractionCreate { interaction })
                .await;
        }
    }

    #[tracing::instrument(skip_all, fields(member_update.old = ?old, member_update.new = ?new, member.tag = %new.user.tag()))]
    async fn guild_member_update(&self, ctx: client::Context, old: Option<Member>, new: Member) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling guild member update event",
            guild_member_update::guild_member_update(ctx, old, new).await
        );
    }

    #[tracing::instrument(skip_all,
        fields(
            msg.id = %event.id,
            msg.channel_id = %event.channel_id,
            msg.author = %event.author.as_ref().map_or(String::new(), |x|x.tag())
        )
    )]
    async fn message_update(
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
    async fn message_delete(
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

    #[tracing::instrument(skip_all, fields(member.tag = %new_member.user.tag()))]
    async fn guild_member_addition(&self, ctx: client::Context, new_member: Member) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling guild_member_addition event",
            guild_member_addition::guild_member_addition(ctx, new_member).await
        );
    }

    #[tracing::instrument(skip_all, fields(member.tag = %user.tag()))]
    async fn guild_member_removal(
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
    async fn reaction_add(&self, ctx: client::Context, event: Reaction) {
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
    async fn reaction_remove(&self, ctx: client::Context, event: Reaction) {
        tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None)
            .unwrap();
        log_error!(
            "Error while handling reaction_remove event",
            reaction_remove::reaction_remove(ctx, event).await
        );
    }
}

async fn unmute(
    ctx: &client::Context,
    config: &Arc<Config>,
    db: &Arc<Db>,
    mute: &robbb_db::mute::Mute,
) -> Result<()> {
    db.set_mute_inactive(mute.id).await?;
    let mut member = config.guild.member(&ctx, mute.user).await?;
    log_error!(member.remove_roles(&ctx, &[config.role_mute]).await);
    log_error!(member.enable_communication(&ctx).await);

    Ok(())
}
