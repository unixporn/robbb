use regex::Regex;
use robbb_util::embeds;

use super::*;

pub static SHOULD_NEVER_TRIGGER_BLOCKLIST: &[&str] = &[
    "",
    "Hello, I am new to linux, and I'd love to get some help with my GNOME installation.",
    "I use Arch with GNOME, but for some reason, my backspace key doesn't work properly. Someone please help",
];

/// Control the blocklist
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    subcommands("blocklist_add", "blocklist_remove", "blocklist_list",)
)]
pub async fn blocklist(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Add a new pattern to the blocklist
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    rename = "add"
)]
pub async fn blocklist_add(
    ctx: Ctx<'_>,
    #[description = "Regex pattern for the blocked word"] pattern: String,
) -> Res<()> {
    let db = ctx.get_db();

    let regex = Regex::new(&pattern).user_error("Illegal regex pattern")?;

    if SHOULD_NEVER_TRIGGER_BLOCKLIST.iter().any(|x| regex.is_match(x)) {
        abort_with!("Pattern matches one of the test strings it should never match. Make sure you're not matching the empty string or anything else you don't want to.")
    }

    db.add_blocklist_entry(ctx.author().id, &pattern).await?;

    ctx.say_success(format!("Added `{}` to the blocklist", pattern)).await?;

    Ok(())
}

/// Remove a pattern from the blocklist
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
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
    ctx.say_success(format!("Removed `{}` from the blocklist", pattern)).await?;

    Ok(())
}

/// Get all blocklist entries
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    rename = "list"
)]
pub async fn blocklist_list(ctx: Ctx<'_>) -> Res<()> {
    let config = ctx.get_config();

    let db = ctx.get_db();
    let entries = db.get_blocklist().await?;

    let is_in_mod_bot_stuff = ctx.channel_id() == config.channel_mod_bot_stuff;
    let embed = embeds::base_embed(ctx.serenity_context())
        .await
        .title("Blocklist")
        .description(entries.iter().map(|x| format!("`{x}`")).join("\n"));
    if is_in_mod_bot_stuff {
        ctx.reply_embed(embed).await?;
    } else {
        ctx.reply_embed_ephemeral(embed).await?;
    }
    Ok(())
}

async fn autocomplete_blocklist_entry(ctx: Ctx<'_>, partial: &str) -> Vec<String> {
    let db = ctx.get_db();
    if let Ok(blocklist) = db.get_blocklist().await {
        blocklist.iter().filter(|x| x.contains(partial)).map(|x| x.to_string()).collect_vec()
    } else {
        Vec::new()
    }
}
