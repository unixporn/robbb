use super::*;

/// Ask a question in tech-support
#[command]
#[only_in(guilds)]
#[usage("ask <question>")]
pub async fn ask(ctx: &client::Context, msg: &Message) -> CommandResult {
    let config = ctx.get_config().await;

    if msg.channel_id != config.channel_tech_support {
        abort_with!("!ask can only be used in the tech-support channel");
    }

    let question_parts = msg.content.split_at_word("!ask");
    let question = question_parts.1.trim();
    let title = util::thread_title_from_text(&question);

    let title = if let Ok(title) = title {
        title
    } else {
        let response = msg.reply_error(&ctx, "You must provide a question").await?;
        tokio::spawn({
            let ctx = ctx.clone();
            let msg = msg.clone();
            let msg_id = msg.id;
            async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let _ = response.delete(&ctx).await;
                let _ = msg.delete(&ctx).await;
            }
            .instrument(tracing::info_span!("delete-invalid-ask-invocation", msg.id = %msg_id))
        });
        return Ok(());
    };

    msg.channel(&ctx)
        .await
        .context("Failed to request message channel")?
        .guild()
        .context("Failed to request guild channel")?
        .create_public_thread(&ctx, msg, |e| e.name(title))
        .await
        .context("Failed to create thread for tech-support question")?;

    Ok(())
}
