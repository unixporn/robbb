use poise::serenity_prelude::{CreateEmbed, Mentionable, User};
use robbb_util::embeds;

use crate::{checks::check_is_moderator, commands};

use super::*;

/// Get general information about any member
#[poise::command(
    guild_only,
    context_menu_command = "Info",
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn menu_info(ctx: Ctx<'_>, user: User) -> Res<()> {
    let member = ctx.guild().unwrap().member(ctx.discord(), &user).await?;
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

async fn make_mod_info_embed(ctx: Ctx<'_>, member: Member) -> Res<CreateEmbed> {
    let db = ctx.get_db();
    let notes = commands::note::fetch_note_values(&db, member.user.id, None).await?;
    let note_counts = notes.iter().counts_by(|x| x.note_type);
    let embed_content = note_counts
        .iter()
        .map(|(note_type, count)| {
            let note_type = match note_type {
                robbb_db::note::NoteType::ManualNote => "Manual notes",
                robbb_db::note::NoteType::BlocklistViolation => "Blocklist violations",
                robbb_db::note::NoteType::Warn => "Warnings",
                robbb_db::note::NoteType::Mute => "Mutes",
                robbb_db::note::NoteType::Ban => "Bans",
                robbb_db::note::NoteType::Kick => "Kicks",
            };
            format!("**{}**: {}", note_type, count)
        })
        .join("\n");

    let mut embed = make_info_embed(ctx, member.clone()).await;
    embed.description(embed_content);
    Ok(embed)
}
