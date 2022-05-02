use tracing_futures::Instrument;

use super::*;

static HOISTING_CHAR: &[char] = &[
    '!', '"', '#', '$', '\'', '(', ')', '*', '-', '+', '.', '/', '=',
];

#[tracing::instrument(skip_all, fields(member_update.old = ?_old, member_update.new = ?new, member.tag = %new.user.tag()))]
pub async fn guild_member_update(
    ctx: client::Context,
    _old: Option<Member>,
    new: Member,
) -> Result<()> {
    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();
    dehoist_member(ctx, new).await?;
    Ok(())
}

pub async fn dehoist_member(ctx: client::Context, member: Member) -> Result<()> {
    let display_name = member.display_name();
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
    member
        .edit(&ctx, |edit| edit.nickname(&cleaned_name))
        .instrument(tracing::info_span!("dehoist-edit-nickname", member.tag = %member.user.tag(), dehoist.old_nick = %display_name, dehoist.new_nick = %cleaned_name))
        .await
        .with_context(|| format!("Failed to rename user {}", member.user.tag()))?;
    Ok(())
}
