use anyhow::*;

use super::*;
use std::str::FromStr;

/// Fetch a users system information.
#[command("fetch")]
#[only_in(guilds)]
#[usage("fetch [user] [field]")]
pub async fn fetch(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let (desired_field, mentioned_user_id) = match args.single_quoted::<String>() {
        Ok(first_arg) => {
            // if first argument is a field, fetch the author's field
            match FetchField::from_str(&first_arg) {
                Ok(field) => (Some(field), msg.author.id),
                Err(_) => {
                    let field = args
                        .single_quoted::<String>()
                        .ok()
                        .map(|x| FetchField::from_str(&x))
                        .transpose()
                        .map_err(|_| UserErr::other("Not a valid fetch field."))?;
                    let user = disambiguate_user_mention(&ctx, &guild, msg, &first_arg)
                        .await?
                        .ok_or(UserErr::MentionedUserNotFound)?;
                    (field, user)
                }
            }
        }
        Err(_) => (args.single_quoted().ok(), msg.author.id),
    };

    let fetch_data = get_fetch_of(&db, mentioned_user_id)
        .await?
        .user_error("This user has not set their fetch.")?;

    let member = guild.member(&ctx, mentioned_user_id).await?;
    let color = member.colour(&ctx).await;

    match desired_field {
        // Handle fetching a single field
        Some(desired_field) => {
            let (field_name, value) = fetch_data
                .into_iter()
                .find(|(k, _)| k == &desired_field)
                .user_error("Failed to get that value. Maybe the user hasn't set it?")?;
            msg.reply_embed(&ctx, |e| {
                e.author(|a| a.name(member.user.tag()).icon_url(member.user.face()));
                e.title(format!("{}'s {}", member.user.name, field_name));
                e.color_opt(color);
                if desired_field == FetchField::Image {
                    e.image(value);
                } else if let Some(value) = format_fetch_field_value(&field_name, value) {
                    e.description(value);
                } else {
                    e.description("Not set");
                }
            })
            .await?;
        }

        // Handle fetching all fields
        None => {
            let profile_data = get_profile_data_of(&db, mentioned_user_id).await?;
            msg.reply_embed(&ctx, |e| {
                e.author(|a| a.name(member.user.tag()).icon_url(member.user.face()));
                e.title(format!("Fetch {}", member.user.tag()));
                e.color_opt(color);

                for (key, value) in fetch_data {
                    if key == FetchField::Image {
                        e.image(value);
                    } else {
                        if key == FetchField::Distro {
                            if let Some(url) = find_distro_image(&value) {
                                e.thumbnail(url);
                            }
                        }
                        if let Some(val) = format_fetch_field_value(&key, value) {
                            e.field(key, val, true);
                        }
                    }
                }
                if let Some(profile) = profile_data {
                    if let Some(git) = profile.git {
                        e.field("git", git, true);
                    }
                    if let Some(desc) = profile.description {
                        e.description(desc);
                    }
                    if let Some(dots) = profile.dotfiles {
                        e.field("dotfiles", dots, true);
                    }
                }
            })
            .await?;
        }
    }

    Ok(())
}

/// load profile values from the database.
/// Returns `None` if no profile values are set
async fn get_profile_data_of(
    db: &Db,
    user_id: UserId,
) -> Result<Option<crate::db::profile::Profile>> {
    let profile = db.get_profile(user_id).await?;
    Ok(profile)
}

/// load fetch values from the database
/// Returns `None` if no fetch is set
async fn get_fetch_of(db: &Db, user_id: UserId) -> Result<Option<Vec<(FetchField, String)>>> {
    let fetch_info = db.get_fetch(user_id).await?;
    let all_data: Vec<(FetchField, String)> = fetch_info
        .map(|x| x.get_values_ordered())
        .unwrap_or_default();
    if all_data.is_empty() {
        Ok(None)
    } else {
        Ok(Some(all_data))
    }
}
