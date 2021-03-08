use super::*;
/// Show the profile-picture of a user.
#[command]
#[usage("pfp [user]")]
pub async fn pfp(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = match args.single::<String>() {
        Ok(mentioned_user) => disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?,
        Err(_) => msg.author.id,
    };

    let member = guild.member(&ctx, mentioned_user_id).await?;
    let color = member.colour(&ctx).await;

    msg.reply_embed(&ctx, |e| {
        e.title(format!("{}'s profile picture", member.user.tag()));
        if let Some(color) = color {
            e.color(color);
        }
        e.image(member.user.avatar_or_default());
    })
    .await?;
    Ok(())
}
