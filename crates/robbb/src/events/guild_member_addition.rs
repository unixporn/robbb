use super::*;
use chrono::{DateTime, Utc};
use std::time::SystemTime;

// TODORW this belongs in crate::commands::mute
/// Adds the mute role to the user, but does _not_ add any database entry.
/// This should only be used if we know that an active database entry for the mute already exists,
/// or else we run the risk of accidentally muting someone forever.
pub async fn set_mute_role(ctx: &client::Context, mut member: Member) -> Result<()> {
    let config = ctx.get_config().await;
    member.add_role(&ctx, config.role_mute).await?;
    Ok(())
}

/// check if there's an active mute of a user that just joined.
/// if so, reapply the mute and log their mute-evasion attempt in modlog
async fn handle_mute_evasion(ctx: &client::Context, new_member: &Member) -> Result<()> {
    let (config, db) = ctx.get_config_and_db().await;
    let active_mute = db.get_active_mute(new_member.user.id).await?;
    if let Some(mute) = active_mute {
        set_mute_role(&ctx, new_member.clone()).await?;
        config
            .channel_modlog
            .send_embed(&ctx, |e| {
                e.author(|a| {
                    a.name("Mute evasion caught")
                        .icon_url(new_member.user.face())
                });
                e.title(new_member.user.name_with_disc_and_id());
                e.description(format!(
                    "User {} was muted and rejoined.\nReadding the mute role.",
                    new_member.mention()
                ));
                e.field("Reason", mute.reason, false);
                e.field("Start", util::format_date_detailed(mute.start_time), false);
                e.field("End", util::format_date_detailed(mute.end_time), false);
            })
            .await?;
    }
    Ok(())
}

#[tracing::instrument(skip_all, fields(member.tag = %new_member.user.tag()))]
pub async fn guild_member_addition(ctx: client::Context, new_member: Member) -> Result<()> {
    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();
    let config = ctx.get_config().await;
    if config.guild != new_member.guild_id {
        return Ok(());
    }

    log_error!(handle_mute_evasion(&ctx, &new_member).await);

    config
        .channel_bot_traffic
        .send_embed(&ctx, |e| {
            let account_created_at = new_member.user.created_at();
            e.author(|a| a.name("Member Join").icon_url(new_member.user.face()));
            e.title(new_member.user.name_with_disc_and_id());
            e.description(format!("User {} joined the server", new_member.mention()));
            if let Some(join_date) = new_member.joined_at {
                e.field(
                    "Account Creation Date",
                    format!(
                        "{} ({})",
                        util::format_date(*account_created_at),
                        util::format_date_before_plaintext(*account_created_at, *join_date)
                            .replace("ago", "before joining")
                    ),
                    false,
                );
                e.field("Join Date", util::format_date(*join_date), false);
            } else {
                e.field(
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
                e.color(serenity::utils::Color::from_rgb(253, 242, 0));
            }
        })
        .await?;
    Ok(())
}
