use chrono::Utc;

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

    let new_content = util::run_text_field_modal(
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
