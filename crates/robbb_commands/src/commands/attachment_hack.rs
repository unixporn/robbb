use std::{borrow::Cow, str::FromStr};

use anyhow::Context;
use reqwest::Url;
use serenity::{
    all::{ChannelId, GuildId, Message, MessageId},
    builder::{CreateAttachment, CreateMessage},
};

use super::*;

#[derive(Debug, Clone)]
pub struct FakeCdnId {
    guild_id: Option<GuildId>,
    channel_id: ChannelId,
    message_id: MessageId,
    latest_url: Option<String>,
}

impl From<&Message> for FakeCdnId {
    fn from(value: &Message) -> Self {
        Self {
            guild_id: value.guild_id,
            channel_id: value.channel_id,
            message_id: value.id,
            latest_url: None,
        }
    }
}

impl FakeCdnId {
    fn pattern() -> &'static regex::Regex {
        lazy_static::lazy_static! {
            static ref PATTERN: regex::Regex = regex::Regex::new(r";;;;fakecdn;;;;(\d*);;;;(\d*);;;;(\d*);;;;(.*);;;;").unwrap();
        }
        &PATTERN
    }
    fn encode(&self) -> String {
        format!(
            ";;;;fakecdn;;;;{};;;;{};;;;{};;;;{};;;;",
            self.guild_id.map(|x| x.get().to_string()).unwrap_or_default(),
            self.channel_id.get(),
            self.message_id.get(),
            self.latest_url.as_deref().unwrap_or_default(),
        )
    }

    pub async fn get_link(&self, ctx: &serenity::client::Context) -> anyhow::Result<String> {
        let message = self.channel_id.message(&ctx, self.message_id).await?;
        message
            .attachments
            .first()
            .map(|x| x.url.clone())
            .ok_or_else(|| anyhow::anyhow!("No attachments found in message {}", self.message_id))
    }
}

impl FromStr for FakeCdnId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let captures = Self::pattern().captures(s).context("invalid fake cdn id")?;
        let (_, [guild_id, channel_id, message_id, latest_url]) = captures.extract();
        let guild_id = guild_id.parse::<GuildId>().ok();
        let channel_id = channel_id.parse::<ChannelId>().context("invalid channel id")?;
        let message_id = message_id.parse::<MessageId>().context("invalid message id")?;
        let latest_url = (!latest_url.is_empty()).then_some(latest_url.to_string());
        Ok(Self { guild_id, channel_id, message_id, latest_url })
    }
}

/// Gather attachments, re-post them in a storage channel, update DB
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }"
)]
pub async fn gather_attachments(
    ctx: Ctx<'_>,
    #[description = "Storage channel"] storage: ChannelId,
) -> Res<()> {
    let db = ctx.get_db();

    let fetches = db.get_all_fetches().await?;
    for fetch in fetches {
        let Some(image_url) = fetch.info.get(&robbb_db::fetch_field::FetchField::Image) else {
            continue;
        };

        if image_url.parse::<FakeCdnId>().is_ok() {
            tracing::info!(user = %fetch.user, "Skipping already-fake CDN image in fetch: {}", image_url);
            continue;
        }

        let url = image_url
            .parse::<Url>()
            .with_context(|| format!("Failed to parse image URL {}", image_url))?;
        let image = reqwest::get(image_url).await?.bytes().await?;

        let image_name =
            url.path_segments().context("broken url")?.last().context("no filename")?;
        let create_attachment =
            CreateAttachment::bytes(Cow::from(image.to_vec()), image_name.to_string());

        let metadata = serde_json::json!({
            "kind": "fetch".to_string(),
            "user_id": fetch.user.get(),
            "original_url": image_url
        });
        let message = storage
            .send_files(
                ctx.http(),
                vec![create_attachment],
                CreateMessage::default().content(serde_json::to_string(&metadata)?),
            )
            .await?;

        let fake_cdn_id = FakeCdnId::from(&message);

        db.update_fetch(
            fetch.user,
            maplit::hashmap! {robbb_db::fetch_field::FetchField::Image => fake_cdn_id.encode() },
        )
        .await?;
    }

    let tag_names = db.list_tags().await?;
    for tag_name in &tag_names {
        let Some(tag) = db.get_tag(tag_name).await? else { continue };

        if FakeCdnId::pattern().is_match(&tag.content) {
            tracing::info!(tag.name = %tag.name, "Skipping already-fake CDN image in tag: {}", tag.content);
            continue;
        }

        let pattern = regex::Regex::new(
            r"\b(https?:\/\/(?:media|cdn).discordapp.(?:net|com)\/attachments\/\d+\/\d*\/.+)\b",
        )
        .unwrap();

        let captures = pattern.captures_iter(&tag.content);
        let urls = captures.map(|x| x.get(1).unwrap().as_str()).collect::<Vec<_>>();
        for (num, url) in urls.iter().enumerate() {
            let attachment_url =
                url.parse::<Url>().with_context(|| format!("Failed to parse image URL {url}"))?;
            let attachment = reqwest::get(attachment_url.clone()).await?.bytes().await?;

            let attachment_name = attachment_url
                .path_segments()
                .with_context(|| format!("broken url {attachment_url}"))?
                .last()
                .context("no filename")?;
            let create_attachment = CreateAttachment::bytes(
                Cow::from(attachment.to_vec()),
                attachment_name.to_string(),
            );

            let metadata = serde_json::json!({
                "kind": "tag".to_string(),
                "nth": num,
                "tag_name": tag_name,
                "original_url": attachment_url,
            });
            let message = storage
                .send_files(
                    ctx.http(),
                    vec![create_attachment],
                    CreateMessage::default().content(serde_json::to_string(&metadata)?),
                )
                .await?;

            let fake_cdn_id = FakeCdnId::from(&message);

            let new_tag_content = tag.content.replace(url, &fake_cdn_id.encode());
            db.set_tag(
                tag.moderator,
                tag.name.to_string(),
                new_tag_content,
                tag.official,
                tag.create_date,
            )
            .await?;
        }
    }

    Ok(())
}
