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
    let title = question
        .lines()
        .find(|x| !x.trim().is_empty())
        .invalid_usage(&ASK_COMMAND_OPTIONS)?;

    let title = if title.len() >= 97 {
        let mut cutoff = 97;
        while !title.is_char_boundary(cutoff) {
            cutoff -= 1;
        }
        format!("{}...", title.split_at(cutoff).0)
    } else {
        title.to_string()
    };

    msg.channel(&ctx)
        .await?
        .guild()
        .context("Failed to request guild channel")?
        .create_public_thread(&ctx, msg, |e| e.name(title))
        .await
        .context("Failed to create thread for tech-support question")?;

    Ok(())
}
