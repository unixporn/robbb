use regex::Regex;

use super::*;

/// Control the blocklist
#[poise::command(
    slash_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator",
    default_member_permissions = "ADMINISTRATOR",
    subcommands("blocklist_add", "blocklist_remove", "blocklist_list",)
)]
pub async fn blocklist(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Add a new pattern to the blocklist
#[poise::command(
    slash_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator",
    rename = "add"
)]
pub async fn blocklist_add(
    ctx: Ctx<'_>,
    #[description = "Regex pattern for the blocked word"] pattern: String,
) -> Res<()> {
    let db = ctx.get_db();

    let _ = Regex::new(&pattern).user_error("Illegal regex pattern")?;

    db.add_blocklist_entry(ctx.author().id, &pattern).await?;

    ctx.say_success(format!("Added `{}` to the blocklist", pattern))
        .await?;

    Ok(())
}

/// Remove a pattern from the blocklist
#[poise::command(
    slash_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator",
    rename = "remove"
)]
pub async fn blocklist_remove(
    ctx: Ctx<'_>,
    #[autocomplete = "autocomplete_blocklist_entry"]
    #[description = "Pattern to remove from the blocklist"]
    pattern: String,
) -> Res<()> {
    let db = ctx.get_db();

    db.remove_blocklist_entry(&pattern).await?;
    ctx.say_success(format!("Removed `{}` from the blocklist", pattern))
        .await?;

    Ok(())
}

/// Get all blocklist entries
#[poise::command(
    slash_command,
    guild_only,
    category = "Moderation",
    check = "crate::checks::check_is_moderator",
    rename = "list"
)]
pub async fn blocklist_list(ctx: Ctx<'_>) -> Res<()> {
    let config = ctx.get_config();

    let db = ctx.get_db();
    let entries = db.get_blocklist().await?;

    let is_in_mod_bot_stuff = ctx.channel_id() == config.channel_mod_bot_stuff;

    ctx.send_embed_full(!is_in_mod_bot_stuff, |e| {
        e.title("Blocklist");
        e.description(entries.iter().map(|x| format!("`{}`", x)).join("\n"));
    })
    .await?;
    Ok(())
}

async fn autocomplete_blocklist_entry(ctx: Ctx<'_>, partial: String) -> Vec<String> {
    let db = ctx.get_db();
    if let Ok(blocklist) = db.get_blocklist().await {
        blocklist
            .iter()
            .filter(|x| x.contains(&partial))
            .map(|x| x.to_string())
            .collect_vec()
    } else {
        Vec::new()
    }
}
