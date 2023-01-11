use std::str::FromStr;

use anyhow::Context;
use poise::{
    serenity_prelude::{
        interaction::{Interaction, InteractionResponseType},
        CollectModalInteraction, MessageId, ReactionType,
    },
    Modal,
};
use serenity::client;
use tracing_futures::Instrument;

use super::*;

#[derive(Debug, poise::Modal)]
#[name = "Ask a question"]
struct AskModal {
    #[name = "Title"]
    #[max_length = 100]
    #[min_length = 1]
    title: String,
    #[name = "Details"]
    #[paragraph]
    #[min_length = 1]
    details: String,
}

/// Ask a question in #tech-support
#[poise::command(
    guild_only,
    slash_command,
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn ask(
    app_ctx: AppCtx<'_>,
    #[description = "Your question"] question: Option<String>,
) -> Res<()> {
    let ctx = Ctx::Application(app_ctx);
    let config = ctx.get_config();

    if ctx.channel_id() != config.channel_tech_support {
        abort_with!("/ask can only be used in the tech-support channel");
    }

    let AskModal { title, details } = AskModal::execute_with_defaults(
        app_ctx,
        AskModal { title: String::new(), details: question.unwrap_or_default() },
    )
    .instrument(tracing::info_span!("wait for modal response"))
    .await?
    .context("Modal timed out")?;

    let webhooks =
        app_ctx.serenity_context().http.get_channel_webhooks(config.channel_tech_support.0).await?;
    let webhook = match webhooks.into_iter().next() {
        Some(webhook) => webhook,
        None => {
            let webhook_json = &serde_json::json!({"name": "Techsupport"});
            app_ctx
                .serenity_context()
                .http
                .create_webhook(config.channel_tech_support.0, webhook_json, None)
                .instrument(tracing::info_span!("create techsupport webhook"))
                .await?
        }
    };

    let post = webhook
        .execute(app_ctx.serenity_context(), true, |w| {
            w.username(&ctx.author().name);
            w.avatar_url(ctx.author().face());
            w.content(format!("**{}**\n{}", title, details))
        })
        .instrument(tracing::debug_span!("execute initial techsupport webhook"))
        .await?
        .context("No post?")?;

    let thread = ctx
        .channel_id()
        .to_channel(&ctx.serenity_context())
        .await
        .context("Failed to request message channel")?
        .guild()
        .context("Failed to request guild channel")?
        .create_public_thread(&ctx.serenity_context(), &post, |e| e.name(title.clone()))
        .await
        .context("Failed to create thread for tech-support question")?;

    thread
        .send_message(&ctx.serenity_context(), |m| {
            m.content(ctx.author().mention());
            m.components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.label("Solved");
                        b.emoji(ReactionType::Unicode("✅".to_string()));
                        b.custom_id(QuestionButtonId(
                            QuestionButtonKind::Solved,
                            ctx.author().id,
                            post.id,
                        ))
                    });
                    r.create_button(|b| {
                        b.label("Edit");
                        b.emoji(ReactionType::Unicode("✏️".to_string()));
                        b.custom_id(QuestionButtonId(
                            QuestionButtonKind::Edit,
                            ctx.author().id,
                            post.id,
                        ))
                    })
                })
            })
        })
        .instrument(tracing::info_span!("Send techsupport ping and button message"))
        .await?;

    Ok(())
}

/// Intercept an interaction and possibly handle it being a techsuport question button press.
/// Returns Ok(true) if it _was_ a techsupport button press, and false otherwise
#[tracing::instrument(skip_all)]
pub async fn handle_ask_button_clicked(
    ctx: &client::Context,
    interaction: &Interaction,
) -> Res<bool> {
    let config = ctx.get_config().await;
    let interaction = match interaction {
        Interaction::MessageComponent(x) => x,
        _ => return Ok(false),
    };
    if !interaction.data.custom_id.starts_with("ask-edit")
        && !interaction.data.custom_id.starts_with("ask-solved")
    {
        return Ok(false);
    }

    let mut control_msg = interaction.message.clone();

    if control_msg.channel_id.to_channel(&ctx).await?.guild().and_then(|x| x.parent_id)
        != Some(config.channel_tech_support)
    {
        return Ok(false);
    }

    let QuestionButtonId(action, asker, post_id) = interaction.data.custom_id.parse()?;

    if interaction.user.id != asker {
        interaction
            .create_interaction_response(&ctx, |ir| {
                ir.kind(InteractionResponseType::ChannelMessageWithSource);
                ir.interaction_response_data(|ir| {
                    ir.content("This is not your question").ephemeral(true)
                })
            })
            .await?;

        return Ok(false);
    }

    let webhooks = ctx.http.get_channel_webhooks(config.channel_tech_support.0).await?;
    let webhook = webhooks.first().context("No webhook for techsupport registered")?;
    let post = ctx.http.get_message(config.channel_tech_support.0, post_id.0).await?;

    let title =
        post.content.lines().next().context("No lines in message")?.trim_matches('*').to_string();
    let details = post.content.lines().skip(1).join("\n");

    if action == QuestionButtonKind::Edit {
        interaction
            .create_interaction_response(&ctx, |ir| {
                *ir = AskModal::create(Some(AskModal { title, details }), String::new());
                ir
            })
            .await?;

        let response = CollectModalInteraction::new(&ctx.shard).author_id(asker).await.unwrap();

        // Send acknowledgement so that the pop-up is closed
        response
            .create_interaction_response(&ctx, |b| {
                b.kind(InteractionResponseType::DeferredUpdateMessage)
            })
            .await?;

        let AskModal { title, details } =
            AskModal::parse(response.data.clone()).map_err(serenity::Error::Other)?;

        webhook
            .edit_message(&ctx, post_id, |m| m.content(format!("**{}**\n{}", title, details)))
            .await?;
    } else if action == QuestionButtonKind::Solved {
        interaction
            .create_interaction_response(&ctx, |ir| {
                ir.kind(InteractionResponseType::DeferredUpdateMessage)
            })
            .await?;

        webhook
            .edit_message(&ctx, post_id, |m| {
                m.content(format!("**[SOLVED] {}**\n{}", title, details));
                m.components(|c| c)
            })
            .await?;

        control_msg.edit(&ctx, |e| e.embed(|e| e.title("Solved!")).components(|c| c)).await?;

        interaction
            .channel_id
            .to_channel(&ctx)
            .await?
            .guild()
            .context("Thread wasn't a guild channel?")?
            .edit_thread(ctx, |e| e.name(format!("[SOLVED] {}", title)).archived(true))
            .await?;
    }
    Ok(true)
}

#[derive(PartialEq, Eq)]
enum QuestionButtonKind {
    Solved,
    Edit,
}

struct QuestionButtonId(QuestionButtonKind, UserId, MessageId);

impl ToString for QuestionButtonId {
    fn to_string(&self) -> String {
        let kind = match self.0 {
            QuestionButtonKind::Solved => "solved",
            QuestionButtonKind::Edit => "edit",
        };
        format!("ask-{}-{}-{}", kind, self.1 .0, self.2 .0)
    }
}

impl FromStr for QuestionButtonId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, rest) = s.split_once('-').context("Malformed QuestionButtonId")?;
        let (kind, rest) = rest.split_once('-').context("Malformed QuestionButton id")?;
        let (user_id, post_id) = rest.split_once('-').context("Malformed QuestionButton id")?;
        Ok(Self(
            match kind {
                "solved" => QuestionButtonKind::Solved,
                "edit" => QuestionButtonKind::Edit,
                _ => anyhow::bail!("Malformed QuestionButtonKind"),
            },
            user_id.parse()?,
            MessageId(post_id.parse()?),
        ))
    }
}
