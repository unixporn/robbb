use crate::embeds::make_create_embed;

use super::*;

/// Show the profile-picture of a user.
#[command]
#[usage("pfp [user]")]
#[only_in(guilds)]
pub async fn pfp(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    //tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();
    let guild = msg.guild(&ctx).context("Failed to load guild")?;

    let mentioned_user_id = match args.single_quoted::<String>() {
        Ok(mentioned_user) => disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?,
        Err(_) => msg.author.id,
    };

    let member = guild.member(&ctx, mentioned_user_id).await?;

    embeds::PaginatedEmbed::create(
        vec![
            embeds::make_create_embed(&ctx, |e| {
                e.title("Server's Profile Picture").image(member.face())
            })
            .await,
            embeds::make_create_embed(&ctx, |e| {
                e.title("User's Profile Picture").image(member.user.face())
            })
            .await,
        ],
        make_create_embed(ctx, |e| e).await,
    )
    .await
    .reply_to(&ctx, &msg)
    .await?;
    tracing::debug!("Replied with pfp");
    Ok(())
}
