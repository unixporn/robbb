use chrono::Utc;
use shared_robbb::prelude::{AppCtx, Ctx, Res};

use super::*;

/// Autocomplete all tags, but also provide whatever the user has already typed as one of the options.
/// Used in /tag set, to provide completion for edits, but also allow adding new tags
async fn tag_autocomplete(ctx: Ctx<'_>, partial: String) -> impl Iterator<Item = String> {
    tag_autocomplete_existing(ctx, partial.clone())
        .await
        .chain(std::iter::once(partial))
}

/// Autocomplete all tags
async fn tag_autocomplete_existing(ctx: Ctx<'_>, partial: String) -> impl Iterator<Item = String> {
    let db = ctx.get_db();
    let tags = match db.list_tags().await {
        Ok(tags) => tags,
        Err(_) => Vec::new(),
    };

    tags.into_iter()
        .filter(move |tag| tag.starts_with(&partial))
        .map(|tag| tag.to_string())
}

pub fn tag_commands() -> Command<UserData, Error> {
    Command {
        subcommands: vec![tag_get(), tag_list(), tag_set(), tag_delete()],
        ..tag()
    }
}

/// Get the text stored in a tag
#[poise::command(slash_command, guild_only, category = "Miscellaneous")]
pub async fn tag(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Get the text stored in a tag
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Miscellaneous",
    rename = "get"
)]
pub async fn tag_get(
    ctx: Ctx<'_>,
    #[description = "The tag to show"]
    #[autocomplete = "tag_autocomplete_existing"]
    #[rename = "tag"]
    tag_name: String,
) -> Res<()> {
    let db = ctx.get_db();

    let tag = db
        .get_tag(&tag_name)
        .await?
        .user_error("No tag with this name exists")?;

    let moderator = tag.moderator.to_user(&ctx.discord()).await?;

    if util::validate_url(&tag.content) {
        ctx.say(&tag.content).await?;
    } else {
        ctx.send_embed(|e| {
            e.title(&tag.name);
            e.description(&tag.content);
            e.footer(|f| f.text(format!("Written by {}", moderator.tag())));
            if let Some(date) = tag.create_date {
                e.timestamp(date);
            }
        })
        .await?;
    }

    Ok(())
}

/// Get the names of all tags
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Miscellaneous",
    rename = "list"
)]
pub async fn tag_list(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();

    let tags = db.list_tags().await?;

    ctx.send_embed(|e| {
        e.title("Tags");
        e.description(&tags.join(", "));
    })
    .await?;

    Ok(())
}

/// Delete a tag
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Miscellaneous",
    rename = "delete"
)]
pub async fn tag_delete(
    ctx: Ctx<'_>,
    #[rename = "name"]
    #[description = "Name of the tag"]
    #[autocomplete = "tag_autocomplete_existing"]
    tag_name: String,
) -> Res<()> {
    let db = ctx.get_db();
    db.delete_tag(tag_name).await?;
    ctx.say_success("Succesfully removed!").await?;
    Ok(())
}

/// Save a new tag or update an old one.
#[poise::command(slash_command, guild_only, category = "Miscellaneous", rename = "set")]
pub async fn tag_set(
    app_ctx: AppCtx<'_>,
    #[rename = "name"]
    #[description = "The name of the tag"]
    #[autocomplete = "tag_autocomplete"]
    tag_name: String,
) -> Res<()> {
    let ctx = Ctx::Application(app_ctx);
    let db = ctx.get_db();

    let existing_tag = db.get_tag(&tag_name).await?;

    let new_content = run_text_field_modal(
        app_ctx,
        "Set your tag",
        "Content",
        "Your tag content",
        &existing_tag.map(|x| x.content).unwrap_or_default(),
    )
    .await?;

    db.set_tag(
        ctx.author().id,
        tag_name,
        new_content,
        true,
        Some(Utc::now()),
    )
    .await?;
    ctx.say_success("Succesfully set!").await?;
    Ok(())
}

pub async fn run_text_field_modal(
    app_ctx: AppCtx<'_>,
    title: &str,
    label: &str,
    placeholder: &str,
    default_content: &str,
) -> Res<String> {
    // For now we must do this manually rather than using the poise macro solution
    // as we want to set a default value for the modal content.

    let interaction = app_ctx.interaction.unwrap();
    interaction
        .create_interaction_response(app_ctx.discord, |ir| {
            ir.kind(poise::serenity_prelude::InteractionResponseType::Modal);
            ir.interaction_response_data(|d| {
                d.custom_id("text_field_modal");
                d.title(title);
                d.components(|c| {
                    c.create_action_row(|r| {
                        r.create_input_text(|t| {
                            t.custom_id("content");
                            t.label(label);
                            t.style(poise::serenity_prelude::InputTextStyle::Paragraph);
                            t.required(true);
                            t.value(default_content);
                            t.placeholder(placeholder)
                        })
                    })
                })
            })
        })
        .await?;

    app_ctx
        .has_sent_initial_response
        .store(true, std::sync::atomic::Ordering::SeqCst);

    // Wait for user to submit
    let response = poise::serenity_prelude::CollectModalInteraction::new(&app_ctx.discord.shard)
        .author_id(interaction.user.id)
        .await
        .unwrap();

    // Send acknowledgement so that the pop-up is closed
    response
        .create_interaction_response(app_ctx.discord, |b| {
            b.kind(poise::serenity_prelude::InteractionResponseType::DeferredUpdateMessage)
        })
        .await?;

    let content = poise::find_modal_text(&mut response.data.clone(), "content")
        .user_error("Missing tag content data from modal")?;
    Ok(content)
}
