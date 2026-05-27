use chrono::Utc;
use poise::serenity_prelude::UserId;
use robbb_util::extensions::{ClientContextExt, CreateEmbedExt, UserExt};
use serenity::{
    all::{AuditLogEntry, audit_log},
    client,
};

pub async fn guild_audit_log_entry_create(
    ctx: client::Context,
    entry: AuditLogEntry,
) -> eyre::Result<()> {
    tracing::info!(
        auditlog.entry.id = %entry.id,
        auditlog.entry.action = ?entry.action,
        auditlog.entry = ?entry,
        "New audit log entry created with action {:?}",
        entry.action
    );

    if entry.user_id == ctx.cache.current_user().id {
        return Ok(());
    }
    let (config, db) = ctx.get_config_and_db().await;
    let user = entry.user_id.to_user(&ctx).await?;
    let Some(target_id) = entry.target_id else { return Ok(()) };
    match entry.action {
        audit_log::Action::Member(audit_log::MemberAction::BanAdd) => {
            let target_user = UserId::new(target_id.get()).to_user(&ctx).await?;
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
            let target_user = UserId::new(target_id.get()).to_user(&ctx).await?;
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
            let target_user = UserId::new(target_id.get()).to_user(&ctx).await?;
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
        audit_log::Action::Message(audit_log::MessageAction::Delete) => {
            // Cache the deleter so message_delete can look it up without polling.
            if let Some(options) = &entry.options
                && let Some(channel_id) = options.channel_id
            {
                let target_user_id = UserId::new(target_id.get());
                let cache = ctx.get_deletion_audit_cache().await;
                cache.insert(channel_id, target_user_id, entry.user_id);
                tracing::debug!(
                    audit_log.channel_id = %channel_id,
                    audit_log.target_user = %target_user_id,
                    audit_log.deleter = %entry.user_id,
                    "Cached message-deletion audit log entry"
                );
            }
        }
        _ => {}
    }
    Ok(())
}
