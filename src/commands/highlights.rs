use super::*;
use crate::Arc;

/// Get notified when someone mentions a word you care about.
#[allow(unreachable_code)]
#[command("highlights")]
#[sub_commands(highlights_add, highlights_get, highlights_remove)]
#[aliases("highlight", "hl")]
#[usage("!highlights <add | get | remove>")]
pub async fn highlights(_: &client::Context, _: &Message) -> CommandResult {
    abort_with!(UserErr::invalid_usage(&HIGHLIGHTS_COMMAND_OPTIONS))
}

/// add a highlight for your user
#[command("add")]
#[usage("!highlights add <word>")]
pub async fn highlights_add(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let trigger = args.message().trim().to_lowercase();
    if trigger.is_empty() {
        abort_with!(UserErr::invalid_usage(&HIGHLIGHTS_COMMAND_OPTIONS));
    } else if trigger.len() < 3 {
        abort_with!("highlight has to be longer than 2 characters");
    }

    let db: Arc<Db> = ctx.get_db().await;
    let max_highlight_cnt = match crate::checks::get_permission_level(ctx, msg).await {
        PermissionLevel::Mod => 20,
        _ => 4,
    };

    let highlights = db.get_highlights().await?;
    let highlights_by_user_cnt = highlights.triggers_for_user(msg.author.id).count();

    if highlights_by_user_cnt >= max_highlight_cnt {
        abort_with!(UserErr::Other(format!(
            "Sorry, you can only watch a maximum of {} highlights",
            max_highlight_cnt
        )));
    }

    msg.author
        .id
        .create_dm_channel(&ctx)
        .await
        .user_error("Couldn't open a DM to you - do you have me blocked?")?
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title("Test to see if you can receive DMs");
                e.description(format!(
                    "If everything went ok, you'll be notified whenever someone says `{}`",
                    trigger
                ))
            })
        })
        .await
        .user_error("Couldn't send you a DM :/\nDo you allow DMs from server members?")?;

    db.set_highlight(msg.author.id, trigger.clone())
        .await
        .user_error("Couldn't add highlight, something went wrong")?;

    msg.reply_success(
        &ctx,
        format!("You will be notified whenever someone says {}", trigger),
    )
    .await?;

    Ok(())
}

/// get all highlights for your user
#[command("get")]
#[aliases("ls", "list")]
#[usage("!highlights get")]
pub async fn highlights_get(ctx: &client::Context, msg: &Message) -> CommandResult {
    let db: Arc<Db> = ctx.get_db().await;
    let highlights = db.get_highlights().await?;

    let highlights_list = highlights.triggers_for_user(msg.author.id).join("\n");

    if highlights_list.is_empty() {
        abort_with!("You don't seem to have set any highlights");
    } else {
        msg.author
            .id
            .create_dm_channel(&ctx)
            .await
            .user_error("Couldn't open a DM to you - do you have me blocked?")?
            .send_message(&ctx, |m| m.embed(|e| e.description(highlights_list)))
            .await
            .user_error("Couldn't send you a DM :/\nDo you allow DMs from server members?")?;
    }
    Ok(())
}

/// removes a highlight
#[command("remove")]
#[aliases("rm", "delete")]
#[usage("!highlights remove <highlight>")]
pub async fn highlights_remove(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let db: Arc<Db> = ctx.get_db().await;
    let trigger = args.message().trim().to_string();
    db.remove_highlight(msg.author.id, trigger.clone())
        .await
        .user_error("Failed to remove the highlight.")?;
    msg.reply_success(
        &ctx,
        format!(
            "You will no longer be notified when someone says '{}'",
            trigger
        ),
    )
    .await?;
    Ok(())
}
