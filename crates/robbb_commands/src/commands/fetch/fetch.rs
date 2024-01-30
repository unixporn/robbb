use std::{collections::HashMap, str::FromStr};

use anyhow::Context;
use robbb_db::{fetch::Fetch, fetch_field::FetchField};
use robbb_util::{cdn_hack::FakeCdnId, embeds};
use serenity::{all::User, builder::CreateEmbedAuthor};

use super::*;

/// Fetch a users system information.
#[autometrics::autometrics(ok_if = Res::is_ok)]
#[poise::command(slash_command, guild_only, rename = "fetch")]
pub async fn fetch(
    ctx: Ctx<'_>,
    #[description = "The user"] user: Option<User>,
    #[description = "The specific field you care about"] field: Option<FetchField>,
) -> Res<()> {
    let guild_id = ctx.guild_id().context("Not in a guild")?;
    let db = ctx.get_db();
    let user = user.unwrap_or_else(|| ctx.author().clone());
    ctx.defer().await?;

    // Query the database
    let fetch_info: Fetch = db.get_fetch(user.id).await?.unwrap_or_else(|| Fetch {
        user: user.id,
        info: HashMap::new(),
        create_date: None,
    });

    let create_date = fetch_info.create_date;
    let fetch_data: Vec<(FetchField, String)> = fetch_info.get_values_ordered();
    let color = if let Ok(member) = guild_id.member(&ctx, user.id).await {
        member.colour(ctx.serenity_context())
    } else {
        None
    };

    match field {
        // Handle fetching a single field
        Some(desired_field) => {
            let (field_name, value) = fetch_data
                .into_iter()
                .find(|(k, _)| k == &desired_field)
                .user_error("Failed to get that value. Maybe the user hasn't set it?")?;
            let mut embed = embeds::base_embed(&ctx)
                .author(CreateEmbedAuthor::new(user.tag()).icon_url(user.face()))
                .title(format!("{}'s {}", user.name, field_name))
                .color_opt(color)
                .timestamp_opt(create_date);
            if desired_field == FetchField::Image {
                let new_link = FakeCdnId::from_str(&value)?.resolve(&ctx).await?;
                embed = embed.image(new_link);
            } else if let Some(value) = format_fetch_field_value(&field_name, value) {
                embed = embed.description(value);
            } else {
                embed = embed.description("Not set");
            }
            ctx.reply_embed(embed).await?;
        }

        // Handle fetching all fields
        None => {
            let mut embed = embeds::base_embed(&ctx)
                .author_user(&user)
                .color_opt(color)
                .timestamp_opt(create_date);

            for (key, value) in fetch_data {
                if key == FetchField::Image {
                    let new_link = FakeCdnId::from_str(&value)?.resolve(&ctx).await?;
                    embed = embed.image(new_link);
                } else {
                    if key == FetchField::Distro {
                        if let Some(url) = find_distro_image(&value) {
                            embed = embed.thumbnail(url);
                        }
                    }
                    embed = embed.field_opt(
                        key.to_string(),
                        format_fetch_field_value(&key, value),
                        true,
                    );
                }
            }
            ctx.reply_embed(embed).await?;
        }
    }

    Ok(())
}
