use super::*;

/// Get the text stored in a tag.
#[command]
#[usage("tag <name> OR tag list")]
#[sub_commands(list_tags, set_tag)]
pub async fn tag(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let db = ctx.get_db().await;
    let tag_name = args.remains().invalid_usage(&TAG_COMMAND_OPTIONS)?;

    let tag = db
        .get_tag(tag_name)
        .await?
        .user_error("No tag with this name exists")?;

    let moderator = tag.moderator.to_user(&ctx).await?;

    if util::validate_url(&tag.content) {
        msg.reply(&ctx, &tag.content).await?;
    } else {
        msg.reply_embed(&ctx, |e| {
            e.title(&tag.name);
            e.description(&tag.content);
            e.footer(|f| f.text(format!("Written by {}", moderator.tag())));
        })
        .await?;
    }

    Ok(())
}

/// Get the names of all tags
#[command("list")]
#[usage("tag list")]
pub async fn list_tags(ctx: &client::Context, msg: &Message) -> CommandResult {
    let db = ctx.get_db().await;

    let tags = db.list_tags().await?;

    msg.reply_embed(&ctx, |e| {
        e.title("Tags");
        e.description(&tags.join(", "));
    })
    .await?;

    Ok(())
}

/// Save a new tag or update an old one.
#[command("settag")]
#[usage("settag <name> <content>")]
pub async fn set_tag(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;
    let tag_name = args
        .single_quoted::<String>()
        .invalid_usage(&SET_TAG_COMMAND_OPTIONS)?;

    let content = args.remains().invalid_usage(&SET_TAG_COMMAND_OPTIONS)?;

    db.set_tag(msg.author.id, tag_name, content.to_string(), true)
        .await?;
    msg.reply_success(&ctx, "Succesfully set!").await?;
    Ok(())
}

/// Save a new tag or update an old one.
#[command("deletetag")]
#[usage("deletetag <name>")]
pub async fn delete_tag(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;
    let tag_name = args
        .single_quoted::<String>()
        .invalid_usage(&SET_TAG_COMMAND_OPTIONS)?;

    db.delete_tag(tag_name).await?;
    msg.reply_success(&ctx, "Succesfully removed!").await?;
    Ok(())
}
