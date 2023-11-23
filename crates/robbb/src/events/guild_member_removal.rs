use robbb_db::Db;

use super::*;

pub async fn guild_member_removal(
    ctx: client::Context,
    guild_id: GuildId,
    user: User,
    member: Option<Member>,
) -> Result<()> {
    let db: Arc<Db> = ctx.get_db().await;
    let config = ctx.get_config().await;
    if config.guild != guild_id {
        return Ok(());
    }

    if let Some(member) = member {
        let roles = member.roles(&ctx).unwrap_or_default();
        if roles.iter().any(|x| x.id == config.role_htm) {
            log_error!(db.add_htm(member.user.id).await);
        }
    }

    config
        .channel_bot_traffic
        .send_embed(&ctx, |e| {
            e.author(|a| a.name("Member Leave").icon_url(user.face()));
            e.title(user.name_with_disc_and_id());
            e.field("Leave Date", util::format_date(chrono::Utc::now()), false);
        })
        .await?;
    db.rm_highlights_of(user.id).await?;
    Ok(())
}
