use robbb_util::extensions::ClientContextExt;
use serenity::{all::ActionExecution, client};

pub async fn execution(ctx: client::Context, execution: ActionExecution) -> anyhow::Result<()> {
    tracing::info!(
        execution.action = ?execution.action,
        execution = ?execution,
        "Automod execution: {:?}",
        execution.action
    );
    if !matches!(execution.action, serenity::all::automod::Action::Alert { .. }) {
        return Ok(());
    }

    let Some(matched_keyword) = execution.matched_keyword else { return Ok(()) };
    let Some(matched_content) = execution.matched_content else { return Ok(()) };
    let Some(message_id) = execution.message_id.or(execution.alert_system_message_id) else {
        return Ok(());
    };
    let Some(channel_id) = execution.channel_id else { return Ok(()) };
    tracing::info!(
        msg.id = %message_id,
        msg.author_id = %execution.user_id,
        msg.channel_id = %channel_id,
        automod.matched_content = %matched_content,
        automod.matched_keyword = %matched_keyword,
        "Automod alerted about message"
    );

    let db = ctx.get_db().await;

    let bot_id = ctx.cache.current_user().id;
    let note_content = format!("Automod deleted message because of word `{matched_content}`");
    db.add_mod_action(
        bot_id,
        execution.user_id,
        note_content,
        chrono::Utc::now(),
        message_id.link(channel_id, Some(execution.guild_id)),
        robbb_db::mod_action::ModActionKind::BlocklistViolation,
    )
    .await?;
    Ok(())
}
