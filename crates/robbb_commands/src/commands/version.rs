use super::*;

/// Get the currently running bot version
#[poise::command(slash_command, guild_only, category = "Bot-Administration", hide_in_help)]
pub async fn version(ctx: Ctx<'_>) -> Res<()> {
    let bot_version = util::BotVersion::get();
    ctx.reply_embed_builder(|e| {
        e.title("Version info")
            .field("profile", bot_version.profile, true)
            .field("commit", bot_version.commit_link(), true)
            .field("message", bot_version.commit_msg, false)
    })
    .await?;

    Ok(())
}
