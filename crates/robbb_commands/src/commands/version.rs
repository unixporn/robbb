

use super::*;

/// Get the currently running bot version
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Bot-Administration",
    hide_in_help
)]
pub async fn version(ctx: Ctx<'_>) -> Res<()> {
    let version = util::bot_version();
    ctx.reply_embed_builder(|e| e.description(format!("Running version `{}`", version))).await?;
    Ok(())
}
