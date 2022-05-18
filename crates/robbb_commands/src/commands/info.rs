use poise::serenity_prelude::{CreateEmbed, Mentionable, User};
use robbb_util::embeds;

use super::*;

/// Get general information about any member
#[poise::command(
    guild_only,
    context_menu_command = "Info",
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn menu_info(ctx: Ctx<'_>, user: User) -> Res<()> {
    let member = ctx.guild().unwrap().member(ctx.discord(), &user).await?;
    let embed = make_info_embed(ctx, member).await;
    ctx.send_embed_full(true, |e| {
        *e = embed;
    })
    .await?;
    Ok(())
}

/// Get general information about any member
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn info(ctx: Ctx<'_>, #[description = "User"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    let embed = make_info_embed(ctx, user).await;
    ctx.send_embed(|e| {
        *e = embed;
    })
    .await?;

    Ok(())
}

async fn make_info_embed(ctx: Ctx<'_>, member: Member) -> CreateEmbed {
    let created_at = member.user.created_at();
    let color = member.colour(ctx.discord());
    embeds::make_create_embed(ctx.discord(), |e| {
        e.title(member.user.tag());
        e.thumbnail(member.user.face());
        e.color_opt(color);
        e.field("ID/Snowflake", member.user.id.to_string(), false);
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
        e
    })
    .await
}
