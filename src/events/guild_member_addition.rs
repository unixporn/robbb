use super::*;
use chrono::{DateTime, Utc};
use std::time::SystemTime;

pub async fn guild_member_addition(
    ctx: client::Context,
    guild_id: GuildId,
    new_member: Member,
) -> Result<()> {
    let config = ctx.get_config().await;
    if config.guild != guild_id {
        return Ok(());
    }

    config
        .channel_bot_traffic
        .send_embed(&ctx, |e| {
            let date = new_member.user.created_at();
            e.author(|a| a.name("Member Join").icon_url(new_member.user.face()));
            e.title(new_member.user.name_with_disc_and_id());
            e.description(format!("User {} joined the server", new_member.mention()));
            e.field(
                "Account Creation Date",
                util::format_date_detailed(date),
                false,
            );
            if let Some(join_date) = new_member.joined_at {
                e.field("Join Date", util::format_date(join_date), false);
            }
            if date
                .signed_duration_since(DateTime::<Utc>::from(SystemTime::now()))
                .num_days()
                <= 3
            {
                e.color(serenity::utils::Color::from_rgb(253, 242, 0));
            }
        })
        .await?;
    Ok(())
}
