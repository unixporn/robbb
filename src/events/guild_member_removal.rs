use super::*;

pub async fn guild_member_removal(
    ctx: client::Context,
    guild_id: GuildId,
    user: User,
    _member: Option<Member>,
) -> Result<()> {
    let db: Arc<Db> = ctx.get_db().await;
    let highlights = db.get_highlights().await?;
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

    for i in highlights
        .iter()
        .filter(|(_, users)| users.contains(&user.id))
    {
        println!("{:#?}", i);
        db.remove_highlight(user.id, i.0.clone()).await?;
    }

    Ok(())
}
