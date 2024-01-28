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
        r"\b(https?://(?:media|cdn)\.discord(?:app)?\.(?:net|com)/(?:ephemeral-)?attachments/\d+/\d*/.+&?)\b"
    ).unwrap();

    pub static ref FAKE_CDN_ID_PATTERN: Regex = Regex::new(
        r";;;;fakecdn;;;;(\d*);;;;(\d*);;;;(\d*);;;;(\d*);;;;(\S*);;;;"
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
    nth_attachment: usize,
    latest_url: Option<String>,
}

impl FakeCdnId {
    pub fn pattern() -> &'static Regex {
        &FAKE_CDN_ID_PATTERN
    }

    /// Create a [`FakeCdnId`] referencing the `nth` attachment of a [`Message`] that contains said attachment.
    pub fn from_message(message: &Message, nth_attachment: usize) -> Self {
        let url = message.attachments.get(nth_attachment).map(|x| x.url.clone());
        Self {
            guild_id: message.guild_id,
            channel_id: message.channel_id,
            message_id: message.id,
            nth_attachment,
            latest_url: url,
        }
    }

    /// Encode this [`FakeCdnId`] into a string.
    pub fn encode(&self) -> String {
        format!(
            ";;;;fakecdn;;;;{};;;;{};;;;{};;;;{};;;;{};;;;",
            self.guild_id.map(|x| x.to_string()).unwrap_or_default(),
            self.channel_id,
            self.message_id,
            self.nth_attachment,
            self.latest_url.as_deref().unwrap_or_default(),
        )
    }

    /// Resolve a [`FakeCdnId`] to a valid attachment url by fetching the mentioned message and getting the first attachment.
    #[tracing::instrument(skip_all, fields(fake_cdn_id = %self))]
    pub async fn resolve(&self, ctx: impl CacheHttp) -> anyhow::Result<String> {
        let message = self.channel_id.message(&ctx, self.message_id).await?;
        message.attachments.get(self.nth_attachment).map(|x| x.url.clone()).ok_or_else(|| {
            anyhow::anyhow!(
                "No {}th attachments found in message {}",
                self.nth_attachment,
                self.message_id
            )
        })
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
        let captures =
            Self::pattern().captures(s).with_context(|| format!("invalid fake cdn id: {s}"))?;
        let (_, [guild_id, channel_id, message_id, nth, latest_url]) = captures.extract();
        let guild_id = guild_id.parse::<GuildId>().ok();
        let channel_id = channel_id.parse::<ChannelId>().context("invalid channel id")?;
        let message_id = message_id.parse::<MessageId>().context("invalid message id")?;
        let nth_attachment = nth.parse::<usize>().context("invalid attachment index")?;
        let latest_url = (!latest_url.is_empty()).then_some(latest_url.to_string());
        Ok(Self { guild_id, channel_id, message_id, nth_attachment, latest_url })
    }
}

/// Download an attachment from the given `attachment_url`,
/// re-upload it to the fake-cdn channel (attaching the given metadata),
/// and return the [`FakeCdnId`] of the uploaded attachment.
#[tracing::instrument(skip_all, fields(%attachment_url, %metadata))]
pub async fn persist_attachment(
    ctx: &serenity::client::Context,
    attachment_url: &str,
    mut metadata: serde_json::Value,
) -> anyhow::Result<FakeCdnId> {
    let config = ctx.get_config().await;

    tracing::info!(%attachment_url, "Persisting attachment in fake cdn: {attachment_url}");
    let attachment_url = attachment_url
        .parse::<reqwest::Url>()
        .with_context(|| format!("Failed to parse attachment URL: `{attachment_url}`"))?;
    let bytes = request_bytes(attachment_url.clone()).await?;
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
    let fake_cdn_id = FakeCdnId::from_message(&message, 0);

    tracing::info!(
        fake_cdn_id = %fake_cdn_id,
        attachment_url = %attachment_url,
        msg.id = %message.id,
        msg.channel_id = %message.channel_id,
        "Persisted attachment in fake cdn: {fake_cdn_id}"
    );

    Ok(fake_cdn_id)
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
        match persist_attachment(ctx, url, metadata).await {
            Ok(fake_cdn_id) => {
                new_string = new_string.replace(url, &fake_cdn_id.encode());
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to persist attachment in fake cdn, continuing");
                continue;
            }
        }
    }
    Ok(new_string)
}

/// Replace all [`FakeCdnId`]s in a string with the corresponding attachment url,
/// fetching a new CDN URL if necessary.
#[tracing::instrument(skip_all, fields(string = %value))]
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

/// Download data from the given `url` and return it as a `Vec<u8>`.
#[tracing::instrument(skip_all)]
async fn request_bytes(url: impl reqwest::IntoUrl) -> reqwest::Result<Vec<u8>> {
    Ok(reqwest::get(url).await?.bytes().await?.to_vec())
}
