use super::*;
use crate::Arc;

/// add a new highlight for the user
#[command("add_highlight")]
#[usage("!add_highlight <word, can have spaces>")]
pub async fn add_highlight(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let args = args.message().trim().to_string();
    if args.contains(" ") {
        msg.reply_error(
            &ctx,
            "Highlight can't contain a space for implementation/performance reasons",
        )
        .await?;
        return Ok(());
    } else if args.len() == 0 {
        msg.reply_error(&ctx, "You must provide a argument. ")
            .await?;
        return Ok(());
    }

    let db: Arc<Db> = ctx.get_db().await;
    match db.set_highlight(msg.author.id, args.clone()).await {
        Ok(_) => {
            msg.reply_success(
                &ctx,
                format!("You will be notified whenever someone says {}", args),
            )
            .await?
        }
        Err(e) => {
            msg.reply_error(&ctx, format!("The database returned a error: {}", e))
                .await?
        }
    };

    Ok(())
}

/// get all highlights for your user
#[command("get_highlights")]
#[usage("!get_highlights")]
pub async fn get_highlights(ctx: &client::Context, msg: &Message) -> CommandResult {
    let db: Arc<Db> = ctx.get_db().await;

    let highlights = db.get_highlights().await?;

    let mut s = String::new();
    'outer: for i in highlights {
        let y = format!("{}\n", i.0);
        for e in i.1 {
            if &msg.author != &e.to_user(ctx).await.unwrap_or_default() {
                continue 'outer;
            }
        }
        s.push_str(&y);
    }
    if s.len() == 0 {
        msg.reply_error(&ctx, "You don't seem seem to have set any highlights.")
            .await?;
    } else {
        msg.reply_success(&ctx, s).await?;
    }
    Ok(())
}

/// removes a highlight
#[command("remove_highlight")]
#[usage("!remove_highlight <highlight, can contain spaces")]
pub async fn remove_highlight(
    ctx: &client::Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    let db: Arc<Db> = ctx.get_db().await;
    args.quoted();
    let args = args.message().to_string();
    db.remove_highlight(msg.author.id, args.clone()).await?;
    msg.reply_success(
        &ctx,
        format!(
            "You will no longer be notified when someone says '{}'",
            args
        ),
    )
    .await?;
    Ok(())
}
