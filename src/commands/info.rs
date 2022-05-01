//use super::*;
use crate::UserData;
/// General information over a user.
//let mentioned_user_id = if let Ok(mentioned_user) = args.single_quoted::<String>() {
//disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
//.await?
//.ok_or(UserErr::MentionedUserNotFound)?
//} else {
//msg.author.id
//};
//let member = guild
//.member(&ctx, mentioned_user_id)
//.await
//.user_error("Failed to load member data, is the user in this server?")?;

//let created_at = mentioned_user_id.created_at();

//let color = member.colour(&ctx).await;

//msg.reply_embed(&ctx, |e| {
//e.title(member.user.tag());
//e.thumbnail(member.user.face());
//e.color_opt(color);
//e.field("ID/Snowflake", mentioned_user_id.to_string(), false);
//e.field(
//"Account creation date",
//util::format_date_detailed(created_at),
//false,
//);
//if let Some(joined_at) = member.joined_at {
//e.field("Join Date", util::format_date_detailed(joined_at), false);
//}

//if !member.roles.is_empty() {
//e.field(
//"Roles",
//member.roles.iter().map(|x| x.mention()).join(" "),
//false,
//);
//}
//})
//.await?;
//Ok(())
//}
use poise::serenity_prelude::Member;
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, UserData, Error>;

#[poise::command(slash_command, prefix_command, track_edits)]
pub async fn poise_info(
    ctx: Context<'_>,
    #[description = "User"] member: Member,
) -> Result<(), Error> {
    ctx.say(format!("kek {}", member.user.tag())).await?;
    Ok(())
}

/// Register application commands in this guild or globally
///
/// Run with no arguments to register in guild, run with argument "global" to register globally.
#[poise::command(prefix_command, hide_in_help)]
pub async fn register(ctx: Context<'_>, #[flag] global: bool) -> Result<(), Error> {
    poise::builtins::register_application_commands(ctx, global).await?;

    Ok(())
}
