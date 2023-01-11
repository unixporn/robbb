use poise::serenity_prelude::{CreateEmbed, Mentionable, User};
use robbb_db::mod_action::ModActionType;
use robbb_util::embeds;

use crate::checks::check_is_moderator;

use super::*;

/// Get general information about any member
#[poise::command(
    guild_only,
    context_menu_command = "Info",
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn menu_info(ctx: Ctx<'_>, user: User) -> Res<()> {
    let member = ctx.guild().unwrap().member(ctx.serenity_context(), &user).await?;
    let embed = if check_is_moderator(ctx).await? {
        make_mod_info_embed(ctx, member).await?
    } else {
        make_info_embed(ctx, member).await
    };
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
    let embed = make_info_embed(ctx, user.clone()).await;
    ctx.send_embed(|e| {
        *e = embed;
    })
    .await?;
    Ok(())
}

/// Get general information and some moderation specific data about any member
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn modinfo(ctx: Ctx<'_>, #[description = "User"] user: Member) -> Res<()> {
    let embed = make_mod_info_embed(ctx, user).await?;
    ctx.send_embed_full(true, |e| {
        *e = embed;
    })
    .await?;

    Ok(())
}

async fn make_info_embed(ctx: Ctx<'_>, member: Member) -> CreateEmbed {
    let created_at = member.user.created_at();
    let color = member.colour(ctx.serenity_context());
    embeds::make_create_embed(ctx.serenity_context(), |e| {
        e.title(member.user.tag());
        e.thumbnail(member.user.face());
        e.color_opt(color);
        e.field("ID/Snowflake", member.user.id.to_string(), false);
        e.field("Account creation date", util::format_date_detailed(*created_at), false);
        if let Some(joined_at) = member.joined_at {
            e.field("Join Date", util::format_date_detailed(*joined_at), false);
        }

        if !member.roles.is_empty() {
            e.field("Roles", member.roles.iter().map(|x| x.mention()).join(" "), false);
        }
        e
    })
    .await
}

async fn make_mod_info_embed(ctx: Ctx<'_>, member: Member) -> Res<CreateEmbed> {
    let db = ctx.get_db();
    let note_counts = db.count_all_mod_actions(member.user.id).await?;
    let embed_content = note_counts
        .iter()
        .map(|(note_type, count)| {
            let note_type = match note_type {
                ModActionType::ManualNote => "Manual notes",
                ModActionType::BlocklistViolation => "Blocklist violations",
                ModActionType::Warn => "Warnings",
                ModActionType::Mute => "Mutes",
                ModActionType::Ban => "Bans",
                ModActionType::Kick => "Kicks",
            };
            format!("**{}**: {}", note_type, count)
        })
        .join("\n");

    let mut embed = make_info_embed(ctx, member.clone()).await;
    embed.description(embed_content);
    Ok(embed)
}
