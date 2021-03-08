use super::*;
/// General information over a user.
#[command]
#[usage("info [user]")]
pub async fn info(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;

    let mentioned_user_id = if let Ok(mentioned_user) = args.single::<String>() {
        disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?
    } else {
        msg.author.id
    };
    let member = guild.member(&ctx, mentioned_user_id).await?;

    let created_at = mentioned_user_id.created_at();
    let join_date = member.joined_at.context("Failed to get join date")?;

    let color = member.colour(&ctx).await;

    msg.reply_embed(&ctx, |e| {
        e.title(member.user.tag());
        e.thumbnail(member.user.avatar_or_default());
        if let Some(color) = color {
            e.color(color);
        }
        e.field("ID/Snowflake", mentioned_user_id.to_string(), false);
        e.field(
            "Account creation date",
            util::format_date(created_at),
            false,
        );
        e.field("Join Date", util::format_date(join_date), false);
        if !member.roles.is_empty() {
            e.field(
                "Roles",
                member.roles.iter().map(|x| x.mention()).join(" "),
                false,
            );
        }
    })
    .await?;

    Ok(())
}
