use super::*;
/// General information over a user.
#[command]
#[only_in(guilds)]
#[usage("info [user]")]
pub async fn info(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(&ctx).context("Failed to load guild")?;

    let mentioned_user_id = if let Ok(mentioned_user) = args.single_quoted::<String>() {
        disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?
    } else {
        msg.author.id
    };
    let member = guild
        .member(&ctx, mentioned_user_id)
        .await
        .user_error("Failed to load member data, is the user in this server?")?;

    let created_at = mentioned_user_id.created_at();

    let color = member.colour(&ctx);

    msg.reply_embed(&ctx, |e| {
        e.title(member.user.tag());
        e.thumbnail(member.user.face());
        e.color_opt(color);
        e.field("ID/Snowflake", mentioned_user_id.to_string(), false);
        e.field(
            "Account creation date",
            util::format_date_detailed(*created_at),
            false,
        );
        if let Some(joined_at) = member.joined_at {
            e.field("Join Date", util::format_date_detailed(*joined_at), false);
        }

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
