use robbb_util::embeds;

use super::*;

/// Show the profile picture of a user.
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn pfp(ctx: Ctx<'_>, #[description = "User"] user: Option<Member>) -> Res<()> {
    let member = member_or_self(ctx, user).await?;
    let server_pfp = member.face();
    let user_pfp = member.user.face();
    let mut embeds = Vec::new();

    if user_pfp != server_pfp {
        embeds.push(
            embeds::base_embed(ctx.serenity_context())
                .await
                .title(format!("{}'s Server Profile Picture", member.user.tag()))
                .image(member.face()),
        );
    }

    embeds.push(
        embeds::base_embed(ctx.serenity_context())
            .await
            .title(format!("{}'s User Profile Picture", member.user.tag()))
            .image(member.user.face()),
    );

    embeds::PaginatedEmbed::create(embeds, embeds::base_embed(ctx.serenity_context()).await)
        .await
        .reply_to(ctx, false)
        .await?;
    Ok(())
}
