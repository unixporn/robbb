use super::*;
use crate::Arc;

/// add a new highlight for the user
#[command("add_highlight")]
#[usage("!add_highlight <word, can have spaces>")]
pub async fn add_highlight(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    args.quoted();
    let args = args.message().to_string();
    let db: Arc<Db> = ctx.get_db().await;
    db.set_highlight(msg.author.id, args.clone()).await?;
    msg.reply_success(&ctx, format!("You will be notified whenever someone mentions '{}'", args)).await?;
    Ok(())
}

/// get all highlights for all users
#[command("get_highlights")]
#[usage("!get_highlights")]
pub async fn get_highlights(ctx: &client::Context, msg: &Message) -> CommandResult {
    let db: Arc<Db> = ctx.get_db().await;
    let highlights = db.get_highlights().await?;
    let mut s = String::new();
    for i in highlights {
        let mut y = format!("{} - ", i.0);
        for x in i.1 {
             y = y + &x.to_user(ctx).await?.tag() + ", ";
        }
        y = y + "\n";
        s.push_str(&y);
    }
    println!("{:#?}", s);
    msg.reply_success(&ctx, s).await?;
    Ok(())
}
