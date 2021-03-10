use super::*;
pub async fn guild_member_addition(
    ctx: client::Context,
    guild_id: GuildId,
    new_member: Member,
) -> Result<()> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    if config.guild != guild_id {
        return Ok(());
    }

    config
        .channel_bot_traffic
        .send_embed(&ctx, |e| {
            e.author(|a| a.name("Member Join").icon_url(new_member.user.face()));
            e.title(new_member.user.name_with_disc_and_id());
            e.description(format!("User {} joined the server", new_member.mention()));
            e.field(
                "Account Creation Date",
                util::format_date_detailed(new_member.user.created_at()),
                false,
            );
            if let Some(join_date) = new_member.joined_at {
                e.field("Join Date", util::format_date(join_date), false);
            }
        })
        .await?;
    Ok(())
}
