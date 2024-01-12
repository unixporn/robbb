use chrono::Utc;
use robbb_util::extensions::{ClientContextExt, CreateEmbedExt, UserExt};
use serenity::{
    all::{audit_log, AuditLogEntry, UserId},
    client,
};

pub async fn guild_audit_log_entry_create(
    ctx: client::Context,
    entry: AuditLogEntry,
) -> anyhow::Result<()> {
    if entry.user_id == ctx.cache.current_user().id {
        return Ok(());
    }
    let (config, db) = ctx.get_config_and_db().await;
    let user = entry.user_id.to_user(&ctx).await?;
    let Some(target_id) = entry.target_id else { return Ok(()) };
    let target_user = UserId::new(target_id.get()).to_user(&ctx).await?;
    match entry.action {
        audit_log::Action::Member(audit_log::MemberAction::BanAdd) => {
            db.add_mod_action(
                user.id,
                target_user.id,
                entry.reason.clone().unwrap_or_default(),
                Utc::now(),
                String::new(),
                robbb_db::mod_action::ModActionKind::Ban,
            )
            .await?;
            config
                .log_bot_action(&ctx, |e| {
                    e.title("Ban")
                        .author_user(&user)
                        .description(format!(
                            "manually yote user: {}",
                            target_user.mention_and_tag()
                        ))
                        .field_opt("Reason", entry.reason, false)
                })
                .await;
        }
        audit_log::Action::Member(audit_log::MemberAction::BanRemove) => {
            config
                .log_bot_action(&ctx, |e| {
                    e.title("Unban")
                        .author_user(&user)
                        .description(format!(
                            "manually unbanned user: {}",
                            target_user.mention_and_tag()
                        ))
                        .field_opt("Reason", entry.reason, false)
                })
                .await;
        }

        audit_log::Action::Member(audit_log::MemberAction::Kick) => {
            db.add_mod_action(
                user.id,
                target_user.id,
                entry.reason.clone().unwrap_or_default(),
                Utc::now(),
                String::new(),
                robbb_db::mod_action::ModActionKind::Kick,
            )
            .await?;
            config
                .log_bot_action(&ctx, |e| {
                    e.title("Kick")
                        .author_user(&user)
                        .description(format!(
                            "manually kicked user: {}",
                            target_user.mention_and_tag()
                        ))
                        .field_opt("Reason", entry.reason, false)
                })
                .await;
        }
        _ => {
            tracing::info!("unhandled audit log entry: {:?}", entry);
        }
    }
    Ok(())
}
