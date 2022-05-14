use anyhow::Context;
use tracing_futures::Instrument;

use super::*;

/// Ask a question in tech-support
#[poise::command(
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn ask(prefix_ctx: PrefixCtx<'_>, #[rest] question: String) -> Res<()> {
    let ctx = Ctx::Prefix(prefix_ctx.clone());
    let config = ctx.get_config();

    if ctx.channel_id() != config.channel_tech_support {
        abort_with!("!ask can only be used in the tech-support channel");
    }

    let question = question.trim();
    let title = util::thread_title_from_text(&question);

    let title = if let Ok(title) = title {
        title
    } else {
        let response = ctx.say_error("You must provide a question").await?;
        let response = response.message().await?;
        let serenity_ctx = ctx.discord().clone();
        let msg = prefix_ctx.msg.clone();
        let msg_id = msg.id;
        tokio::spawn({
            async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let _ = response.delete(&serenity_ctx).await;
                let _ = msg.delete(&serenity_ctx).await;
            }
            .instrument(tracing::info_span!("delete-invalid-ask-invocation", msg.id = %msg_id))
        });
        return Ok(());
    };

    ctx.guild_channel()
        .await?
        .create_public_thread(&ctx.discord(), prefix_ctx.msg, |e| e.name(title))
        .await
        .context("Failed to create thread for tech-support question")?;

    Ok(())
}

/*
#[poise::command(guild_only, slash_command, category = "Miscellaneous", rename = "ask")]
pub async fn ask_slash(ctx: Ctx<'_>) -> Res<()> {
    let config = ctx.get_config();
    ctx.send_embed_full(true, |e| {
        e.description(format!(
            "Please ask questions in {} using !ask instead:\n!ask <title>\n<more details>",
            config.channel_tech_support.mention()
        ));
    })
    .await?;
    Ok(())
}
*/

/*
use poise::Modal;

/// Ask a question in tech-support
#[poise::command(guild_only, slash_command, category = "Miscellaneous")]
pub async fn ask_slash(app_ctx: AppCtx<'_>) -> Res<()> {
    #[derive(Debug, Modal)]
    #[name = "Ask a question"]
    struct AskModal {
        #[name = "Title"]
        title: String,
        #[name = "Details"]
        #[paragraph]
        details: String,
    }

    let ctx = Ctx::Application(app_ctx.clone());
    let config = ctx.get_config();

    if ctx.channel_id() != config.channel_tech_support {
        abort_with!("!ask can only be used in the tech-support channel");
    }

    let AskModal { title, details } = AskModal::execute(app_ctx).await?;

    let handle = ctx
        .send_embed(|e| {
            e.author_user(ctx.author().clone());
            e.title(title.clone());
            e.description(details);
        })
        .await?;
    let post = handle.message().await?;

    ctx.channel_id()
        .to_channel(&ctx.discord())
        .await
        .context("Failed to request message channel")?
        .guild()
        .context("Failed to request guild channel")?
        .create_public_thread(&ctx.discord(), post, |e| e.name(title))
        .await
        .context("Failed to create thread for tech-support question")?;

    Ok(())
}
*/
