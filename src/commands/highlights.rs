use super::*;
use crate::Arc;

#[command("highlights")]
#[sub_commands(highlights_add, highlights_get, highlights_remove)]
#[aliases("highlight")]
#[usage("!highlights <add | get | remove>")]
pub async fn highlights(_: &client::Context, _: &Message) -> CommandResult {
    abort_with!(UserErr::invalid_usage(&HIGHLIGHTS_COMMAND_OPTIONS))
}

/// add a highlight for your user
#[command("add")]
#[usage("!highlights add <word>")]
pub async fn highlights_add(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let trigger_word = args.message().trim().to_string();
    if trigger_word.contains(' ') {
        abort_with!("Highlights must not contain spaces");
    } else if trigger_word.is_empty() {
        abort_with!("You must provide a argument");
    } else if trigger_word.len() < 3 {
        abort_with!("highlight has to be larger than 2 characters");
    }

    let db: Arc<Db> = ctx.get_db().await;
    let max_highlight_cnt: u8 = match crate::checks::get_permission_level(ctx, msg)
        .await
        .unwrap_or(PermissionLevel::User)
    {
        PermissionLevel::Mod => 20,
        _ => 4,
    };

    let highlights = db.get_highlights().await?;
    let highlights_by_user_cnt = highlights
        .iter()
        .filter(|(_, users)| users.contains(&msg.author.id))
        .count();

    if highlights_by_user_cnt as u8 >= max_highlight_cnt {
        abort_with!(UserErr::Other(format!(
            "Sorry, you can only watch a maximum of {} highlights",
            max_highlight_cnt
        )));
    }
    db.set_highlight(msg.author.id, trigger_word.clone())
        .await
        .user_error("Something went wrong")?;

    msg.author
        .id
        .create_dm_channel(&ctx)
        .await
        .user_error("Couldn't open a DM to you - do you have DMs enabled?")?
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title("Highlight added");
                e.description(format!(
                    "Notifying you whenever someone says `{}`",
                    trigger_word
                ))
            })
        })
        .await
        .user_error(
            "Couldn't send you a DM :/\n\
    Did you change your DM settings recently? ",
        )?;

    msg.reply_success(
        &ctx,
        format!(
            "You will be notified whenever someone says {}",
            trigger_word
        ),
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

    let highlights_list = highlights
        .iter()
        .filter(|(_, users)| users.contains(&msg.author.id))
        .map(|(word, _)| word)
        .join("\n");

    // yes yes, we are checking the length of the text, whatever
    if highlights_list.is_empty() {
        abort_with!(UserErr::Other(
            "You don't seem to have set any highlights".to_string()
        ));
    } else {
        msg.reply_embed(&ctx, |e| {
            e.description(highlights_list);
        })
        .await?;
    }
    Ok(())
}

/// removes a highlight
#[command("remove")]
#[aliases("rm", "delete")]
#[usage("!highlights remove <highlight>")]
pub async fn highlights_remove(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let db: Arc<Db> = ctx.get_db().await;
    let trigger_word = args.message().to_string();
    db.remove_highlight(msg.author.id, trigger_word.clone())
        .await
        .user_error("Failed to remove the highlight.")?;
    msg.reply_success(
        &ctx,
        format!(
            "You will no longer be notified when someone says '{}'",
            trigger_word
        ),
    )
    .await?;
    Ok(())
}
