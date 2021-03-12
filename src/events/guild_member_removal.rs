use super::*;

pub async fn guild_member_removal(
    ctx: client::Context,
    guild_id: GuildId,
    user: User,
    _member: Option<Member>,
) -> Result<()> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
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
    Ok(())
}
