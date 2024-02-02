use super::*;
use chrono::{DateTime, Utc};
use poise::serenity_prelude::{Member, Mentionable};
use robbb_commands::commands;
use robbb_util::{
    embeds,
    extensions::{ClientContextExt, UserExt},
    log_error, util,
};
use serenity::builder::CreateEmbedAuthor;
use std::time::SystemTime;

#[tracing::instrument(skip_all)]
async fn handle_htm_evasion(ctx: &client::Context, new_member: &Member) -> Result<()> {
    let (config, db) = ctx.get_config_and_db();
    let is_htm = db.check_user_htm(new_member.user.id).await?;
    if is_htm {
        tracing::info!("Re-adding hard-to-moderate-role due to htm evasion");
        config
            .channel_modlog
            .send_embed_builder(&ctx, |e| {
                e.author(
                    CreateEmbedAuthor::new("HTM evasion caught").icon_url(new_member.user.face()),
                )
                .title(new_member.user.name_with_disc_and_id())
                .description(format!(
                    "User {} was HTM and rejoined.\nRe-applying HTM role.",
                    new_member.mention()
                ))
            })
            .await?;
        new_member.add_role(&ctx.http, config.role_htm).await?;
    }
    Ok(())
}

/// check if there's an active mute of a user that just joined.
/// if so, reapply the mute and log their mute-evasion attempt in modlog
#[tracing::instrument(skip_all)]
async fn handle_mute_evasion(ctx: &client::Context, new_member: &Member) -> Result<()> {
    let (config, db) = ctx.get_config_and_db();
    let active_mute = db.get_active_mute(new_member.user.id).await?;
    if let Some(mute) = active_mute {
        tracing::info!("Re-adding mute-role due to mute evasion");
        commands::mute::set_mute_role(&ctx, new_member.clone()).await?;
        let embed = embeds::base_embed(ctx.user_data())
            .author(CreateEmbedAuthor::new("Mute evasion caught").icon_url(new_member.user.face()))
            .title(new_member.user.name_with_disc_and_id())
            .description(format!(
                "User {} was muted and rejoined.\nReadding the mute role.",
                new_member.mention()
            ))
            .field("Reason", mute.reason, false)
            .field("Start", util::format_date_detailed(mute.start_time), false)
            .field("End", util::format_date_detailed(mute.end_time), false);
        config.channel_modlog.send_embed(&ctx, embed).await?;
    }
    Ok(())
}

pub async fn guild_member_addition(ctx: &client::Context, new_member: &Member) -> Result<()> {
    tracing::info!(user.id = %new_member.user.id, user.name = %new_member.user.tag(), "Handling guild_member_addtion");
    let config = ctx.get_config();
    if config.guild != new_member.guild_id {
        return Ok(());
    }

    log_error!(handle_htm_evasion(&ctx, &new_member).await);
    log_error!(handle_mute_evasion(&ctx, &new_member).await);

    let account_created_at = new_member.user.created_at();
    config
        .channel_bot_traffic
        .send_embed_builder(&ctx, |mut e| {
            e = e
                .author(CreateEmbedAuthor::new("Member Join").icon_url(new_member.user.face()))
                .title(new_member.user.name_with_disc_and_id())
                .description(format!("User {} joined the server", new_member.mention()));
            if let Some(join_date) = new_member.joined_at {
                e = e.field(
                    "Account Creation Date",
                    format!(
                        "{} ({})",
                        util::format_date(*account_created_at),
                        util::format_date_before_plaintext(*account_created_at, *join_date)
                            .replace("ago", "before joining")
                    ),
                    false,
                );
                e = e.field("Join Date", util::format_date(*join_date), false);
            } else {
                e = e.field(
                    "Account Creation Date",
                    util::format_date_detailed(*account_created_at),
                    false,
                );
            }
            if DateTime::<Utc>::from(SystemTime::now())
                .signed_duration_since(*account_created_at)
                .num_days()
                <= 3
            {
                e = e.color(serenity::all::Colour::from_rgb(253, 242, 0));
            }
            e
        })
        .await?;
    Ok(())
}
