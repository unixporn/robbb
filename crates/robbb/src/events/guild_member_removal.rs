use robbb_db::Db;
use serenity::builder::CreateEmbedAuthor;

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

    tracing::info!(
        user.id = %user.id,
        user.name = %user.tag(),
        "Handling guild_member_removal event for user {}",
        user.tag(),
    );

    if let Some(member) = member {
        let roles = member.roles(&ctx).unwrap_or_default();
        let is_htm = db.check_user_htm(member.user.id).await?; // check if already htm is added to DB

        if roles.iter().any(|x| x.id == config.role_htm) && !is_htm {
            // add htm if not in db already
            log_error!(db.add_htm(member.user.id).await);
        } else {
            // remove htm from db if user doesn't have htm anymore
            log_error!(db.remove_htm(member.user.id).await);
        }
    }

    config
        .channel_bot_traffic
        .send_embed_builder(&ctx, |e| {
            e.author(CreateEmbedAuthor::new("Member Leave").icon_url(user.face()))
                .title(user.name_with_disc_and_id())
                .field("Leave Date", util::format_date(chrono::Utc::now()), false)
        })
        .await?;
    db.rm_highlights_of(user.id).await?;
    Ok(())
}
