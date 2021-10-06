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
    let color = member.colour(&ctx);

    msg.reply_embed(&ctx, |e| {
        e.title(format!("{}'s profile picture", member.user.tag()));
        e.color_opt(color);
        e.image(member.user.face());
    })
    .await?;

    tracing::debug!("Replied with pfp");
    Ok(())
}
