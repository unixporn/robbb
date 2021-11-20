use serenity::builder::CreateEmbed;

use crate::embeds::basic_create_embed;

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
            create_pfp_embed(&ctx, "Server's Profile Picture", member.face()).await,
            create_pfp_embed(&ctx, "User's Profile Picture", member.user.face()).await,
        ],
        basic_create_embed(ctx).await,
    )
    .await
    .reply_to(&ctx, &msg)
    .await?;
    tracing::debug!("Replied with pfp");
    Ok(())
}

async fn create_pfp_embed(ctx: &client::Context, title: &str, image_url: String) -> CreateEmbed {
    let mut e = embeds::basic_create_embed(&ctx).await;
    e.title(title).image(image_url);
    e
}
