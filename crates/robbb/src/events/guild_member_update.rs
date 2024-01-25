use serenity::builder::EditMember;
use tracing_futures::Instrument;

use super::*;

static HOISTING_CHAR: &[char] = &['!', '"', '#', '$', '\'', '(', ')', '*', '-', '+', '.', '/', '='];

pub async fn guild_member_update(
    ctx: client::Context,
    _old: Option<Member>,
    new: Option<Member>,
    event: GuildMemberUpdateEvent,
) -> Result<()> {
    let (config, db) = ctx.get_config_and_db().await;
    if let Some(new) = new {
        dehoist_member(ctx.clone(), new.clone()).await?;
    }

    if event.roles.iter().any(|x| *x == config.role_htm) {
        log_error!(db.add_htm(event.user.id).await);
    }

    Ok(())
}

pub async fn dehoist_member(ctx: client::Context, mut member: Member) -> Result<()> {
    let display_name = member.display_name().to_string();
    if !display_name.starts_with(HOISTING_CHAR) {
        return Ok(());
    }
    let cleaned_name = display_name.trim_start_matches(HOISTING_CHAR);
    // If the users name is _exclusively_ hoisting chars, just prepend a couple "z"s to put them at the very bottom.
    let cleaned_name = if cleaned_name.is_empty() {
        format!("zzz{}", display_name)
    } else {
        cleaned_name.to_string()
    };
    tracing::info!(user.old_name = %display_name, user.cleaned_name = %cleaned_name, "Dehoisting user");
    let tag = member.user.tag();
    member
        .edit(&ctx, EditMember::default().nickname(&cleaned_name))
        .instrument(tracing::info_span!("dehoist-edit-nickname", member.tag = %tag, dehoist.old_nick = %display_name, dehoist.new_nick = %cleaned_name))
        .await
        .with_context(|| format!("Failed to rename user {tag}"))?;
    Ok(())
}
