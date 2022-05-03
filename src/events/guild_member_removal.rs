use super::*;

#[tracing::instrument(skip_all, fields(member.tag = %user.tag()))]
pub async fn guild_member_removal(
    ctx: client::Context,
    guild_id: GuildId,
    user: User,
    _member: Option<Member>,
) -> Result<()> {
    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();
    let db: Arc<Db> = ctx.get_db().await;
    let config = ctx.get_config().await;
    if config.guild != guild_id {
        return Ok(());
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
