use anyhow::Context;
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
    let guild = ctx.guild().context("Not in a guild")?.to_owned();
    let member = guild.member(ctx.serenity_context(), &user).await?;
    let embed = if check_is_moderator(ctx).await? {
        make_mod_info_embed(ctx, member.as_ref()).await?
    } else {
        make_info_embed(ctx, member.as_ref()).await
    };
    ctx.reply_embed_ephemeral(embed).await?;
    Ok(())
}

/// Get general information about any member
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn info(ctx: Ctx<'_>, #[description = "User"] user: Option<Member>) -> Res<()> {
    let user = member_or_self(ctx, user).await?;
    ctx.reply_embed(make_info_embed(ctx, &user).await).await?;
    Ok(())
}

/// Get general information and some moderation specific data about any member
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn modinfo(ctx: Ctx<'_>, #[description = "User"] user: Member) -> Res<()> {
    ctx.reply_embed_ephemeral(make_mod_info_embed(ctx, &user).await?).await?;
    Ok(())
}

async fn make_info_embed(ctx: Ctx<'_>, member: &Member) -> CreateEmbed {
    let created_at = member.user.created_at();
    let color = member.colour(ctx.serenity_context());
    let mut e = embeds::base_embed(ctx.serenity_context())
        .await
        .title(member.user.tag())
        .thumbnail(member.user.face())
        .color_opt(color)
        .field("ID/Snowflake", member.user.id.to_string(), false)
        .field("Account creation date", util::format_date_detailed(*created_at), false)
        .field_opt("Join Date", member.joined_at.map(|x| util::format_date_detailed(*x)), false);

    if !member.roles.is_empty() {
        e = e.field("Roles", member.roles.iter().map(|x| x.mention()).join(" "), false);
    }
    e
}

async fn make_mod_info_embed(ctx: Ctx<'_>, member: &Member) -> Res<CreateEmbed> {
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

    Ok(make_info_embed(ctx, member).await.description(embed_content))
}
