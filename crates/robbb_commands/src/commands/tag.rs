use anyhow::Context;
use chrono::Utc;
use poise::Modal;
use robbb_util::cdn_hack;
use tracing_futures::Instrument;

use super::*;

/// Get the text stored in a tag
#[poise::command(slash_command, guild_only)]
pub async fn tag(
    ctx: Ctx<'_>,
    #[description = "The tag to show"]
    #[autocomplete = "tag_autocomplete_existing"]
    #[rename = "tag"]
    tag_name: String,
) -> Res<()> {
    let db = ctx.get_db();

    let tag = db.get_tag(&tag_name).await?.user_error("No tag with this name exists")?;

    let moderator = tag.moderator.to_user(&ctx.serenity_context()).await?;

    let content =
        cdn_hack::resolve_cdn_links_in_string(ctx.serenity_context(), &tag.content).await?;
    if util::validate_url(&content) {
        ctx.say(content).await?;
    } else {
        ctx.reply_embed_builder(|e| {
            e.title(&tag.name)
                .description(content)
                .footer_str(format!("Written by {}", moderator.tag()))
                .timestamp_opt(tag.create_date)
        })
        .await?;
    }

    Ok(())
}

/// Get the names of all tags
#[poise::command(slash_command, guild_only, rename = "taglist")]
pub async fn taglist(ctx: Ctx<'_>) -> Res<()> {
    let db = ctx.get_db();

    let tags = db.list_tags().await?;
    ctx.reply_embed_builder(|e| e.title("Tags").description(tags.join(", "))).await?;
    Ok(())
}

/// Manage tags
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    subcommands("tag_set", "tag_delete")
)]
pub async fn settag(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Delete a tag
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
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

#[derive(Debug, poise::Modal)]
#[name = "Tag"]
struct TagModal {
    #[name = "Content"]
    #[placeholder = "Content of your tag"]
    #[paragraph]
    content: String,
}

/// Save a new tag or update an old one.
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    rename = "set"
)]
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
    // Content to pre-fill into the modal text field
    let default_content = existing_tag.map(|x| x.content).unwrap_or_default();

    let result = TagModal::execute_with_defaults(app_ctx, TagModal { content: default_content })
        .instrument(tracing::info_span!("wait for modal response"))
        .await?
        .context("Modal timed out")?;

    let content = cdn_hack::persist_cdn_links_in_string(
        ctx.serenity_context(),
        &result.content,
        serde_json::json!({"kind": "tag", "tag_name": tag_name}),
    )
    .await?;
    db.set_tag(ctx.author().id, tag_name, content, true, Some(Utc::now())).await?;
    ctx.say_success("Succesfully set!").await?;
    Ok(())
}

/// Autocomplete all tags, but also provide whatever the user has already typed as one of the options.
/// Used in /tag set, to provide completion for edits, but also allow adding new tags
async fn tag_autocomplete(ctx: Ctx<'_>, partial: &str) -> impl Iterator<Item = String> {
    let last = if partial.is_empty() { vec![] } else { vec![partial.to_string()] };
    tag_autocomplete_existing(ctx, partial).await.chain(last).dedup() // when the partial fully matches a value, we otherwise get a duplicate
}

/// Autocomplete all tags
async fn tag_autocomplete_existing(ctx: Ctx<'_>, partial: &str) -> impl Iterator<Item = String> {
    let db = ctx.get_db();
    let tags = db.list_tags().await.unwrap_or_default();
    let partial = partial.to_ascii_lowercase();
    tags.into_iter().filter(move |tag| tag.to_ascii_lowercase().starts_with(&partial))
}
