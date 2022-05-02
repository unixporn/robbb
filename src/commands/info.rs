use super::*;

#[poise::command(slash_command, prefix_command, track_edits)]
pub async fn info(ctx: Ctx<'_>, #[description = "User"] user: Member) -> Result<(), Error> {
    let created_at = user.user.created_at();

    let color = user.colour(ctx.discord());
    util::reply_embed(ctx, |e| {
        e.title(user.user.tag());
        e.thumbnail(user.user.face());
        e.color_opt(color);
        e.field("ID/Snowflake", user.user.id.to_string(), false);
        e.field(
            "Account creation date",
            util::format_date_detailed(*created_at),
            false,
        );
        if let Some(joined_at) = user.joined_at {
            e.field("Join Date", util::format_date_detailed(*joined_at), false);
        }

        if !user.roles.is_empty() {
            e.field(
                "Roles",
                user.roles.iter().map(|x| x.mention()).join(" "),
                false,
            );
        }
    })
    .await?;

    Ok(())
}
