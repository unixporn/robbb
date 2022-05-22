use anyhow::{Context, Result};
use robbb_util::config::Config;
use std::{
    os::unix::prelude::MetadataExt,
    path::{Path, PathBuf},
};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use serenity::{
    futures::{self, future::try_join_all, TryStreamExt},
    model::{
        channel::Attachment,
        id::{ChannelId, MessageId},
    },
};

#[tracing::instrument(skip_all, fields(msg.id = %msg_id, msg.channel_id = %channel_id))]
pub async fn store_attachments(
    attachments: impl IntoIterator<Item = Attachment>,
    msg_id: MessageId,
    channel_id: ChannelId,
    attachment_cache_path: PathBuf,
) -> Result<()> {
    let dirname = generate_dirname(channel_id, msg_id);
    let attachment_dir_path = attachment_cache_path.join(dirname);
    tokio::fs::create_dir_all(&attachment_dir_path).await?;

    try_join_all(
        attachments
            .into_iter()
            .map(|attachment| store_single_attachment(attachment_dir_path.clone(), attachment)),
    )
    .await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(%attachment.url, %attachment.size, %attachment.filename, ?attachment.content_type))]
/// Store a single attachment in the given directory path.
async fn store_single_attachment(dir_path: impl AsRef<Path>, attachment: Attachment) -> Result<()> {
    let file_path = dir_path.as_ref().join(attachment.filename);
    tracing::debug!("Storing file {}", &file_path.display());

    let resp = reqwest::get(&attachment.url).await.context("Failed to load attachment")?;
    let mut body = resp
        .bytes_stream()
        .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
        .into_async_read()
        .compat();

    let mut attachment_file =
        tokio::fs::File::create(file_path).await.context("Failed to create attachment log file")?;

    tokio::io::copy(&mut body, &mut attachment_file).await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
/// Search for logged attachments for a given message.
pub async fn find_attachments_for(
    attachment_cache_path: impl AsRef<Path>,
    channel_id: ChannelId,
    msg_id: MessageId,
) -> Result<Vec<(PathBuf, tokio::fs::File)>> {
    let attachment_cache_path = attachment_cache_path.as_ref();
    let attachment_dir = attachment_cache_path.join(generate_dirname(channel_id, msg_id));
    if !attachment_dir.exists() {
        return Ok(Vec::new());
    }
    let mut read_dir = tokio::fs::read_dir(attachment_dir).await?;

    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        if entry.file_type().await?.is_file() {
            let file = tokio::fs::File::open(entry.path()).await?;
            entries.push((entry.path(), file));
        }
    }

    Ok(entries)
}

#[tracing::instrument(skip_all)]
/// Restrict the disk-space used up by attachment logs by removing old files.
pub async fn cleanup(config: &Config) -> Result<()> {
    let mut read_dir = tokio::fs::read_dir(&config.attachment_cache_path).await?;

    let mut files = Vec::new();

    let mut total_size_bytes = 0usize;
    while let Some(entry) = read_dir.next_entry().await? {
        let mut read_attachments = tokio::fs::read_dir(entry.path()).await?;
        while let Some(attachment) = read_attachments.next_entry().await? {
            if attachment.file_type().await?.is_file() {
                let metadata = attachment.metadata().await?;
                total_size_bytes += metadata.size() as usize;
                files.push((attachment, metadata));
            }
        }
    }

    if total_size_bytes > config.attachment_cache_max_size {
        files.sort_by_key(|(_, meta)| meta.modified().expect("Unsupported platform"));
    }

    while total_size_bytes > config.attachment_cache_max_size && !files.is_empty() {
        let (file, meta) = files.remove(0);
        tracing::debug!("deleting {}", file.path().display());
        tokio::fs::remove_file(file.path()).await?;
        total_size_bytes -= meta.size() as usize;
    }
    Ok(())
}

fn generate_dirname(channel_id: ChannelId, msg_id: MessageId) -> String {
    format!("{}-{}", channel_id, msg_id)
}
