use shared_robbb::fetch_field::FetchField;

use super::*;

/// Fetch a users system information.
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Miscellaneous",
    rename = "fetch"
)]
pub async fn fetch(
    ctx: Ctx<'_>,
    #[description = "The user"] user: Option<Member>,
    #[description = "The specific field you care about"] field: Option<FetchField>,
) -> Res<()> {
    let db = ctx.get_db();
    let user = member_or_self(ctx, user).await?;

    // Query the database
    let fetch_info = db
        .get_fetch(user.user.id)
        .await?
        .user_error("This user has not set their fetch.")?;

    let create_date = fetch_info.create_date;
    let fetch_data: Vec<(FetchField, String)> = fetch_info.get_values_ordered();
    let color = user.colour(&ctx.discord());

    match field {
        // Handle fetching a single field
        Some(desired_field) => {
            let (field_name, value) = fetch_data
                .into_iter()
                .find(|(k, _)| k == &desired_field)
                .user_error("Failed to get that value. Maybe the user hasn't set it?")?;
            ctx.send_embed(|e| {
                e.author(|a| a.name(user.user.tag()).icon_url(user.user.face()));
                e.title(format!("{}'s {}", user.user.name, field_name));
                e.color_opt(color);
                if let Some(date) = create_date {
                    e.timestamp(date);
                }
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
            let profile = db.get_profile(user.user.id).await?;
            ctx.send_embed(|e| {
                e.author_user(user.user.clone());
                e.title(format!("Fetch {}", user.user.tag()));
                e.color_opt(color);
                if let Some(date) = create_date {
                    e.timestamp(date);
                }

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
                if let Some(git) = profile.git {
                    e.field("git", git, true);
                }
                if let Some(desc) = profile.description {
                    e.description(desc);
                }
                if let Some(dots) = profile.dotfiles {
                    e.field("dotfiles", dots, true);
                }
            })
            .await?;
        }
    }

    Ok(())
}
