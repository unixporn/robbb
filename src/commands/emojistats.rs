use super::*;

#[command]
#[usage("top [field-name] [`regex`]")]
pub async fn emojistats(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let field_name = args.single_quoted::<String>().ok();

    let emojis = db.get

    todo!();
}
