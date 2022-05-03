use super::*;

/// Get the currently running bot version
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Miscellaneous",
    hide_in_help,
    track_edits
)]
pub async fn version(ctx: Ctx<'_>) -> Res<()> {
    let version = util::bot_version();
    ctx.send_embed(|e| {
        e.description(format!("Running version `{}`", version));
    })
    .await?;
    Ok(())
}
