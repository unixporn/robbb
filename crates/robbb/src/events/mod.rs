use std::sync::Arc;

use anyhow::{Context, Result};

use robbb_db::Db;
use robbb_util::extensions::*;
use robbb_util::{config::Config, log_error, prelude::Error, util, UserData};
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

pub async fn handle_event(
    ctx: &client::Context,
    event: &poise::Event<'_>,
    _framework: &poise::Framework<UserData, Error>,
    data: UserData,
) {
    use poise::Event::*;
    let result = match event.clone() {
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
        GuildMemberAddition { new_member } => {
            guild_member_addition::guild_member_addition(ctx.clone(), new_member).await
        }
        GuildMemberRemoval {
            guild_id,
            user,
            member_data_if_available,
        } => {
            guild_member_removal::guild_member_removal(
                ctx.clone(),
                guild_id,
                user,
                member_data_if_available,
            )
            .await
        }
        Message { new_message } => {
            message_create::message_create(ctx.clone(), data, new_message).await
        }
        MessageUpdate {
            old_if_available,
            new,
            event,
        } => message_update::message_update(ctx.clone(), data, old_if_available, new, event).await,
        MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id,
        } => {
            message_delete::message_delete(ctx.clone(), channel_id, deleted_message_id, guild_id)
                .await
        }

        MessageDeleteBulk {
            multiple_deleted_messages_ids,
            channel_id,
            guild_id,
        } => {
            message_delete::message_delete_bulk(
                ctx.clone(),
                channel_id,
                multiple_deleted_messages_ids,
                guild_id,
            )
            .await
        }
        ReactionAdd { add_reaction } => reaction_add::reaction_add(ctx.clone(), add_reaction).await,
        ReactionRemove { removed_reaction } => {
            reaction_remove::reaction_remove(ctx.clone(), removed_reaction).await
        }

        _ => Ok(()),
    };

    log_error!(
        format!("Error while handling {} event", event.name(),),
        result
    );
}

async fn unmute(
    ctx: &client::Context,
    config: &Arc<Config>,
    db: &Arc<Db>,
    mute: &robbb_db::mute::Mute,
) -> Result<()> {
    db.set_mute_inactive(mute.id).await?;
    let mut member = config.guild.member(&ctx, mute.user).await?;
    member.remove_roles(&ctx, &[config.role_mute]).await?;

    Ok(())
}
