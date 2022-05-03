use poise::serenity_prelude::Mentionable;

use super::*;

/// Get general information about any member
#[poise::command(slash_command, guild_only, prefix_command, category = "Miscellaneous", track_edits)]
pub async fn info(ctx: Ctx<'_>, #[description = "User"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let created_at = user.user.created_at();

    let color = user.colour(ctx.discord());
    ctx.send_embed(|e| {
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
