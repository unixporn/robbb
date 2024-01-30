use super::*;
use anyhow::Context;
use poise::{serenity_prelude::ReactionType, CreateReply, Modal};
use regex::Regex;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

lazy_static::lazy_static! {
    static ref POLL_OPTION_START_OF_LINE_PATTERN: Regex = Regex::new(r"^\s*-|^\s*\d\.|^\s*\*").unwrap();
}

/// Get people to vote on your question
#[poise::command(slash_command, guild_only, subcommands("poll_vote", "poll_multi"))]
pub async fn poll(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Get people to vote on your yes/no question
#[poise::command(slash_command, guild_only, rename = "vote")]
pub async fn poll_vote(
    ctx: Ctx<'_>,
    #[description = "A yes/no question"] question: String,
) -> Res<()> {
    if question.len() > 255 {
        abort_with!("The question is too long :(")
    }

    let poll_msg = ctx
        .reply_embed_builder(|e| {
            e.author_user(ctx.author())
                .title("Poll")
                .description(question.clone())
                .footer_str(format!("From: {}", ctx.author().tag()))
        })
        .await?;

    let poll_msg = poll_msg.message().await?;
    poll_msg.react(&ctx.serenity_context(), ReactionType::Unicode("✅".to_string())).await?;
    poll_msg.react(&ctx.serenity_context(), ReactionType::Unicode("🤷".to_string())).await?;
    poll_msg.react(&ctx.serenity_context(), ReactionType::Unicode("❎".to_string())).await?;

    let config = ctx.get_config();
    if ctx.channel_id() == config.channel_mod_polls {
        poll_msg
            .create_thread(
                ctx.serenity_context(),
                util::thread_title_from_text(&question).unwrap_or_else(|_| "Poll".to_string()),
            )
            .await?;
    }
    Ok(())
}

#[derive(Debug, Modal)]
#[name = "Set up a poll"]
struct MultiPollModal {
    #[name = "Title"]
    #[min_length = 2]
    #[max_length = 100]
    #[placeholder = "Which color has the best personality?"]
    title: String,
    #[name = "Options"]
    #[placeholder = "- Red\n- Green\n- Blue\n- Yellow-ish Turquoise"]
    #[paragraph]
    options: String,
}

/// Have others select one of many options.
#[poise::command(slash_command, guild_only, rename = "multi")]
pub async fn poll_multi(app_ctx: AppCtx<'_>) -> Res<()> {
    let ctx = poise::Context::Application(app_ctx);

    let modal_result = MultiPollModal::execute(app_ctx).await?.context("Modal timed out")?;

    let options_lines = modal_result.options.lines().collect_vec();

    if options_lines.len() > SELECTION_EMOJI.len() || options_lines.len() < 2 {
        abort_with!(UserErr::new(format!(
            "There must be between 2 and {} options",
            SELECTION_EMOJI.len()
        )))
    }

    let options_lines = options_lines
        .into_iter()
        .map(|line| POLL_OPTION_START_OF_LINE_PATTERN.replace(line, "").to_string());

    let options = SELECTION_EMOJI.iter().zip(options_lines).collect_vec();

    let poll_msg = ctx
        .send(CreateReply::default().embed({
            let mut e = CreateEmbed::default().title("Poll").description(&modal_result.title);
            for (emoji, option) in options.iter() {
                e = e.field(format!("Option {}", emoji), option, false);
            }
            e.footer(CreateEmbedFooter::new(format!("from: {}", ctx.author().tag())))
        }))
        .await?;
    let poll_msg = poll_msg.message().await?;

    for (emoji, _) in options.into_iter() {
        poll_msg.react(&ctx.serenity_context(), ReactionType::Unicode(emoji.to_string())).await?;
    }
    poll_msg.react(&ctx.serenity_context(), ReactionType::Unicode("🤷".to_string())).await?;

    let config = ctx.get_config();
    if ctx.channel_id() == config.channel_mod_polls {
        poll_msg
            .create_thread(
                ctx.serenity_context(),
                util::thread_title_from_text(&modal_result.title)
                    .unwrap_or_else(|_| "Poll".to_string()),
            )
            .await?;
    }
    Ok(())
}
