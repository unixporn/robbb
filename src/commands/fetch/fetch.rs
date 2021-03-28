use crate::extensions::*;
use anyhow::*;

use super::*;

/// Fetch a users system information.
#[command("fetch")]
#[usage("fetch [user] [field]")]
pub async fn fetch(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let mentioned_user_id = match args.single_quoted::<String>() {
        Ok(mentioned_user) => disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?,
        Err(_) => msg.author.id,
    };

    let desired_field = args.single_quoted::<String>().ok();

    let all_data = get_fetch_and_profile_data_of(&db, mentioned_user_id)
        .await?
        .user_error("This user has not set their fetch :/")?;

    let member = guild.member(&ctx, mentioned_user_id).await?;
    let color = member.colour(&ctx).await;

    match desired_field {
        // Handle fetching a single field
        Some(desired_field) => {
            let (field_name, value) = all_data.into_iter().find(|(k, _)| str::eq_ignore_ascii_case(k, &desired_field))
                .user_error("Failed to get that value. Maybe the user hasn't set it, or maybe the field does not exist?")?;

            msg.reply_embed(&ctx, |e| {
                e.author(|a| a.name(member.user.tag()).icon_url(member.user.face()));
                e.title(format!("{}'s {}", member.user.name, field_name));
                e.color_opt(color);
                if str::eq_ignore_ascii_case(&desired_field, IMAGE_KEY) {
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
            msg.reply_embed(&ctx, |e| {
                e.author(|a| a.name(member.user.tag()).icon_url(member.user.face()));
                e.title(format!("Fetch {}", member.user.tag()));
                e.color_opt(color);

                for (key, value) in all_data {
                    if key == DISTRO_KEY {
                        if let Some(image_url) = find_distro_image(&value) {
                            e.thumbnail(image_url);
                        }
                    }
                    if key == IMAGE_KEY {
                        e.image(value);
                    } else if key == DESCRIPTION_KEY {
                        e.description(value);
                    } else if let Some(value) = format_fetch_field_value(&key, value) {
                        e.field(key, value, true);
                    }
                }
            })
            .await?;
        }
    }

    Ok(())
}

/// load all data shown in fetch, including profile values from the database.
/// Returns `None` if neither a fetch or any profile values are set.
async fn get_fetch_and_profile_data_of(
    db: &Db,
    user_id: UserId,
) -> Result<Option<Vec<(String, String)>>> {
    let (profile, fetch_info) = tokio::try_join!(db.get_profile(user_id), db.get_fetch(user_id))?;

    let mut all_data: Vec<(String, String)> = fetch_info
        .map(|x| x.get_values_ordered())
        .unwrap_or_default();
    if let Some(profile) = profile {
        all_data.extend(profile.into_values_map());
    }
    if all_data.is_empty() {
        Ok(None)
    } else {
        Ok(Some(all_data))
    }
}
