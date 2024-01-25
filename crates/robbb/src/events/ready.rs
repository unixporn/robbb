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

async fn start_mute_handler(ctx: client::Context) {
    let (config, db) = ctx.get_config_and_db().await;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let mutes = match db.get_newly_expired_mutes().await {
                Ok(mutes) => mutes,
                Err(err) => {
                    tracing::error!(error.message = %err, "Failed to request expired mutes: {}", err);
                    continue;
                }
            };
            for mute in mutes {
                if let Err(err) = unmute(&ctx, &config, &db, &mute).await {
                    tracing::error!(error.message = %err, "Error handling mute removal: {}", err);
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
