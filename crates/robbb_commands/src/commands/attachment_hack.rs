use robbb_util::cdn_hack::{self, FakeCdnId};

use super::*;

/// Gather attachments, re-post them in a storage channel, update DB
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn gather_attachments(ctx: Ctx<'_>) -> Res<()> {
    ctx.defer().await?;
    let db = ctx.get_db();

    let fetches = db.get_all_fetches().await?;
    for fetch in fetches {
        let Some(image_url) = fetch.info.get(&robbb_db::fetch_field::FetchField::Image) else {
            continue;
        };

        if image_url.parse::<FakeCdnId>().is_ok() {
            tracing::info!(user = %fetch.user, "Skipping already-fake CDN image in fetch: {image_url}");
            continue;
        }

        let metadata = serde_json::json!({
            "kind": "fetch".to_string(),
            "user_id": fetch.user.get(),
            "original_url": image_url
        });

        let fake_cdn_id =
            cdn_hack::persist_attachment(ctx.serenity_context(), image_url, metadata).await?;

        db.update_fetch(
            fetch.user,
            maplit::hashmap! {robbb_db::fetch_field::FetchField::Image => fake_cdn_id.encode() },
        )
        .await?;
    }

    let tag_names = db.list_tags().await?;
    for tag_name in &tag_names {
        let Some(tag) = db.get_tag(tag_name).await? else { continue };

        let metadata = serde_json::json!({"kind": "tag", "tag_name": tag_name});
        let new_content =
            cdn_hack::persist_cdn_links_in_string(ctx.serenity_context(), &tag.content, metadata)
                .await?;

        if new_content != tag.content {
            db.set_tag(
                tag.moderator,
                tag.name.to_string(),
                new_content,
                tag.official,
                tag.create_date,
            )
            .await?;
        }
    }
    ctx.say_success("Successfully went through fetch and tag data and re-uploaded attachments!")
        .await?;

    Ok(())
}
