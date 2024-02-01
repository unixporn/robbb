use robbb_commands::modlog;
use serenity::futures::StreamExt;

use super::*;

pub async fn ready(ctx: client::Context, _data_about_bot: Ready) -> Result<()> {
    let config = ctx.get_config().await;

    let bot_version = util::BotVersion::get();
    tracing::info!(
        version.profile = %bot_version.profile,
        version.commit_hash = %bot_version.commit_hash,
        version.commit_msg = %bot_version.commit_msg,
        "Robbb is ready!"
    );

    let _ = config
        .channel_mod_bot_stuff
        .send_embed_builder(&ctx, |e| {
            e.title("Hey guys, I'm back!")
                .field("profile", bot_version.profile, true)
                .field("commit", bot_version.commit_link(), true)
                .field("message", bot_version.commit_msg, false)
        })
        .await;

    dehoist_everyone(ctx.clone(), config.guild).await;

    start_mute_handler(ctx.clone()).await;
    start_attachment_log_handler(ctx).await;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn dehoist_everyone(ctx: client::Context, guild_id: GuildId) {
    guild_id
        .members_iter(&ctx)
        .filter_map(|x| async { x.ok() })
        .for_each_concurrent(None, |member| async {
            log_error!(
                "Error while dehoisting a member",
                guild_member_update::dehoist_member(ctx.clone(), member).await
            );
        })
        .await;
}

/// End a given mute, ending the users timeout and removing the mute role,
/// as well as setting the mute to inactive in the db.
#[tracing::instrument(skip_all, fields(user.id = %mute.user, mute.id = %mute.id))]
async fn unmute(ctx: &client::Context, mute: &robbb_db::mute::Mute) -> Result<()> {
    let (config, db) = ctx.get_config_and_db().await;
    db.set_mute_inactive(mute.id).await?;
    let mut member = config.guild.member(&ctx, mute.user).await?;
    log_error!(member.remove_roles(&ctx, &[config.role_mute]).await);
    log_error!(member.enable_communication(&ctx).await);
    Ok(())
}

async fn start_mute_handler(ctx: client::Context) {
    let db = ctx.get_db().await;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let mutes = match db.get_newly_expired_mutes().await {
                Ok(mutes) => mutes,
                Err(err) => {
                    tracing::error!(error.message = %err, "Failed to request expired mutes");
                    continue;
                }
            };
            for mute in mutes {
                tracing::info!(
                    user.id = %mute.user,
                    mute.id = %mute.id,
                    mute.end_time = %mute.end_time,
                    "Mute expired for user {}, unmuting", mute.user
                );
                if let Err(err) = unmute(&ctx, &mute).await {
                    tracing::error!(
                        error.message = %err,
                        error = ?err,
                        mute.id = %mute.id,
                        user.id = %mute.user,
                        "Error handling mute removal"
                    );
                } else {
                    modlog::log_user_mute_ended(&ctx, &mute).await;
                }
            }
        }
    });
}

async fn start_attachment_log_handler(ctx: client::Context) {
    let config = ctx.get_config().await;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60 * 5)).await;

            log_error!(
                "Failed to clean up attachments",
                crate::attachment_logging::cleanup(&config).await
            );
        }
    });
}
