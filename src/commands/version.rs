use super::*;

/// Get the currently running bot version
#[command]
#[usage("version")]
#[help_available(false)]
pub async fn version(ctx: &client::Context, msg: &Message) -> CommandResult {
    let version = util::bot_version();
    msg.reply_embed(&ctx, |e| {
        e.description(format!("Running version `{}`", version));
    })
    .await?;
    Ok(())
}
