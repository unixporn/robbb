use regex::Regex;

use super::*;

/// Control the blocklist
#[allow(unreachable_code)]
#[command]
#[only_in(guilds)]
#[sub_commands(blocklist_add, blocklist_remove, blocklist_get)]
#[usage("blocklist <add | get | remove>")]
pub async fn blocklist(_ctx: &client::Context, _msg: &Message) -> CommandResult {
    abort_with!(UserErr::invalid_usage(&BLOCKLIST_COMMAND_OPTIONS));
}

#[command("add")]
#[usage("blocklist add `regex`")]
pub async fn blocklist_add(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let pattern = args
        .remains()
        .filter(|x| x.starts_with('`') && x.ends_with('`'))
        .user_error("Invalid argument. The word should be surrounded by \"`\"")?;

    // verified previously
    let pattern = pattern
        .strip_prefix('`')
        .and_then(|x| x.strip_suffix('`'))
        .unwrap();

    let _ = Regex::new(&pattern).user_error("Illegal regex pattern")?;

    db.add_blocklist_entry(msg.author.id, &pattern).await?;

    msg.reply_success(&ctx, format!("Added `{}` to the blocklist", pattern))
        .await?;

    Ok(())
}

#[command("remove")]
#[aliases("rm", "delete")]
#[usage("blocklist remove `regex`")]
pub async fn blocklist_remove(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let pattern = args
        .remains()
        .filter(|x| x.starts_with('`') && x.ends_with('`'))
        .user_error("Invalid argument. The word should be surrounded by \"`\"")?;

    // verified previously
    let pattern = pattern
        .strip_prefix('`')
        .and_then(|x| x.strip_suffix('`'))
        .unwrap();

    db.remove_blocklist_entry(pattern).await?;
    msg.reply_success(&ctx, format!("Removed `{}` from the blocklist", pattern))
        .await?;

    Ok(())
}

#[command("get")]
#[aliases("ls", "list")]
#[usage("blocklist get")]
pub async fn blocklist_get(ctx: &client::Context, msg: &Message) -> CommandResult {
    let (config, db) = ctx.get_config_and_db().await;

    if msg.channel_id != config.channel_mod_bot_stuff {
        abort_with!("This can only be used in the mod-internal bot channel");
    }

    let entries = db.get_blocklist().await?;

    msg.reply_embed(&ctx, |e| {
        e.description(entries.iter().map(|x| format!("`{}`", x)).join("\n"));
    })
    .await?;
    Ok(())
}
