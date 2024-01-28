use std::{borrow::Cow, str::FromStr};

use anyhow::Context;
use regex::Regex;
use serenity::{
    all::{ChannelId, GuildId, Message, MessageId},
    builder::{CreateAttachment, CreateMessage},
    http::CacheHttp,
};

use crate::extensions::ClientContextExt;

lazy_static::lazy_static! {
    pub static ref CDN_LINK_PATTERN: Regex = Regex::new(
        r"\b(https?://(?:media|cdn)\.discord(?:app)?\.(?:net|com)/attachments/\d+/\d*/.+&?)\b"
    ).unwrap();

    pub static ref FAKE_CDN_ID_PATTERN: Regex = Regex::new(
        r";;;;fakecdn;;;;(\d*);;;;(\d*);;;;(\d*);;;;(\S*);;;;"
    ).unwrap();
}

/// ID referencing a message that contains an attachment.
/// We use these IDs rather than direct attachment links, because discord CDN links expire after a short while.
/// With these IDs, we can re-fetch a new, valid CDN link whenever we need to.
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
    pub fn pattern() -> &'static Regex {
        &FAKE_CDN_ID_PATTERN
    }

    pub fn encode(&self) -> String {
        format!(
            ";;;;fakecdn;;;;{};;;;{};;;;{};;;;{};;;;",
            self.guild_id.map(|x| x.to_string()).unwrap_or_default(),
            self.channel_id,
            self.message_id,
            self.latest_url.as_deref().unwrap_or_default(),
        )
    }

    #[tracing::instrument(skip_all, fields(fake_cdn_id = %self))]
    pub async fn resolve(&self, ctx: impl CacheHttp) -> anyhow::Result<String> {
        let message = self.channel_id.message(&ctx, self.message_id).await?;
        message
            .attachments
            .first()
            .map(|x| x.url.clone())
            .ok_or_else(|| anyhow::anyhow!("No attachments found in message {}", self.message_id))
    }
}

impl std::fmt::Display for FakeCdnId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.encode())
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

pub async fn persist_attachment(
    ctx: &serenity::client::Context,
    attachment_url: &str,
    mut metadata: serde_json::Value,
) -> anyhow::Result<FakeCdnId> {
    let config = ctx.get_config().await;
    let attachment_url = attachment_url
        .parse::<reqwest::Url>()
        .with_context(|| format!("Failed to parse image URL {attachment_url}"))?;
    let bytes = reqwest::get(attachment_url.clone()).await?.bytes().await?;
    let attachment_name = attachment_url
        .path_segments()
        .context("Couldn't get path segments from URL")?
        .last()
        .context("no filename in attachment url")?;

    let create_attachment =
        CreateAttachment::bytes(Cow::from(bytes.to_vec()), attachment_name.to_string());

    metadata["original_url"] = serde_json::Value::String(attachment_url.to_string());

    let message = config
        .channel_fake_cdn
        .send_files(
            &ctx.http,
            vec![create_attachment],
            CreateMessage::default().content(serde_json::to_string(&metadata)?),
        )
        .await?;

    Ok(FakeCdnId::from(&message))
}

/// Go through a string, find all discord cdn links (Links matching [`CDN_LINK_PATTERN`]).
/// Download the attachments, re-upload them to the fake-cdn channel (attaching the given metadata),
/// and replace the links in the string with the corresponding [`FakeCdnId`].
#[tracing::instrument(skip_all, fields(%string, %metadata))]
pub async fn persist_cdn_links_in_string(
    ctx: &serenity::client::Context,
    string: &str,
    metadata: serde_json::Value,
) -> anyhow::Result<String> {
    let mut new_string = string.to_string();

    let captures = CDN_LINK_PATTERN.captures_iter(string);
    for (num, capture) in captures.enumerate() {
        let url = capture.get(1).unwrap().as_str();
        let mut metadata = metadata.clone();
        metadata["num"] = serde_json::Value::Number(num.into());
        let fake_cdn_id = persist_attachment(ctx, url, metadata).await?;
        new_string = new_string.replace(url, &fake_cdn_id.encode());
    }
    Ok(new_string)
}

/// Replace all [`FakeCdnId`]s in a string with the corresponding attachment url,
/// fetching a new CDN URL if necessary.
pub async fn resolve_cdn_links_in_string(
    ctx: impl CacheHttp,
    value: &str,
) -> anyhow::Result<String> {
    let mut new_value = value.to_string();
    for mat in FakeCdnId::pattern().find_iter(value) {
        let mat = mat.as_str();
        let fake_cdn_id = mat.parse::<FakeCdnId>()?;
        let new_link = fake_cdn_id.resolve(&ctx).await?;
        new_value = new_value.replace(mat, &new_link);
    }
    Ok(new_value)
}
