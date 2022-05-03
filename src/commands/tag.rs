use chrono::Utc;
use poise::Modal;

use super::*;

// TODORW possibly return AutocompleteChoice s here, to preview a bit of the tags content
async fn tag_autocomplete(ctx: Ctx<'_>, partial: String) -> impl Iterator<Item = String> {
    let db = ctx.get_db();
    let tags = match db.list_tags().await {
        Ok(tags) => tags,
        Err(_) => Vec::new(),
    };

    tags.into_iter()
        .filter(move |tag| tag.starts_with(&partial))
        .map(|tag| tag.to_string())
}

/// Get the text stored in a tag
#[poise::command(slash_command, guild_only, category = "Miscellaneous", track_edits)]
pub async fn tag(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Get the text stored in a tag
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Miscellaneous",
    rename = "get",
    track_edits
)]
pub async fn tag_get(
    ctx: Ctx<'_>,
    #[description = "The tag to show"]
    #[autocomplete = "tag_autocomplete"]
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

#[derive(Debug, Modal)]
#[name = "Specify the tags content"]
struct TagSetModal {
    #[name = "Content"]
    #[paragraph]
    content: String,
}

/// Save a new tag or update an old one.
#[poise::command(slash_command, guild_only, category = "Miscellaneous", rename = "set")]
pub async fn tag_set(
    ctx: AppCtx<'_>,
    #[rename = "name"]
    #[description = "The name of the tag"]
    tag_name: String,
) -> Res<()> {
    let result = TagSetModal::execute(ctx).await?;

    let ctx = Ctx::Application(ctx);
    let db = ctx.get_db();

    db.set_tag(
        ctx.author().id,
        tag_name,
        result.content.to_string(),
        true,
        Some(Utc::now()),
    )
    .await?;
    ctx.say_success("Succesfully set!").await?;
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
    #[autocomplete = "tag_autocomplete"]
    tag_name: String,
) -> Res<()> {
    let db = ctx.get_db();
    db.delete_tag(tag_name).await?;
    ctx.say_success("Succesfully removed!").await?;
    Ok(())
}
