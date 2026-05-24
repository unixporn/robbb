use eyre::{Context, Result};
use robbb_util::config::Config;
use std::{
    os::unix::prelude::MetadataExt,
    path::{Path, PathBuf},
    time::Instant,
};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tracing_futures::Instrument;

use serenity::{
    futures::{self, TryStreamExt, future::try_join_all},
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
    let started_at = Instant::now();
    let dirname = generate_dirname(channel_id, msg_id);
    let attachment_dir_path = attachment_cache_path.join(dirname);
    tokio::fs::create_dir_all(&attachment_dir_path).await?;

    let stored_bytes = try_join_all(
        attachments
            .into_iter()
            .map(|attachment| store_single_attachment(attachment_dir_path.clone(), attachment)),
    )
    .await?;

    let stored_files = stored_bytes.len() as u64;
    let total_stored_bytes = stored_bytes.into_iter().sum::<u64>();
    metrics::counter!(crate::monitoring::ATTACHMENT_CACHE_FILES_STORED_TOTAL)
        .increment(stored_files);
    metrics::counter!(crate::monitoring::ATTACHMENT_CACHE_BYTES_STORED_TOTAL)
        .increment(total_stored_bytes);
    metrics::histogram!(crate::monitoring::ATTACHMENT_CACHE_STORE_SIZE_BYTES)
        .record(total_stored_bytes as f64);
    metrics::histogram!(crate::monitoring::ATTACHMENT_CACHE_STORE_DURATION_MS)
        .record(started_at.elapsed().as_secs_f64() * 1000.0);

    Ok(())
}

#[tracing::instrument(skip_all, fields(%attachment.url, %attachment.size, %attachment.filename, ?attachment.content_type))]
/// Store a single attachment in the given directory path.
async fn store_single_attachment(
    dir_path: impl AsRef<Path>,
    attachment: Attachment,
) -> Result<u64> {
    let file_path = dir_path.as_ref().join(attachment.filename);
    tracing::debug!(file_path = %file_path.display(), "Storing file {}", &file_path.display());

    let resp = reqwest::get(&attachment.url).await.context("Failed to load attachment")?;
    let mut body =
        resp.bytes_stream().map_err(futures::io::Error::other).into_async_read().compat();

    let mut attachment_file =
        tokio::fs::File::create(file_path).await.context("Failed to create attachment log file")?;

    let bytes_written = tokio::io::copy(&mut body, &mut attachment_file).await?;
    Ok(bytes_written)
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
    let mut total_files = 0u64;
    let mut total_size_bytes = 0u64;
    let max_size_bytes = config.attachment_cache_max_size as u64;

    while let Some(entry) = read_dir.next_entry().await? {
        let mut read_attachments = tokio::fs::read_dir(entry.path()).await?;
        while let Some(attachment) = read_attachments.next_entry().await? {
            if attachment.file_type().await?.is_file() {
                let metadata = attachment.metadata().await?;
                total_size_bytes += metadata.size();
                total_files += 1;
                files.push((attachment, metadata));
            }
        }
    }

    if total_size_bytes > max_size_bytes {
        files.sort_by_key(|(_, meta)| meta.modified().expect("Unsupported platform"));
    }

    tracing::info!(attachment_logs.total_size_bytes = %total_size_bytes, attachment_logs.total_files = %total_files, "Performing attachment cleanup");

    while total_size_bytes > max_size_bytes && !files.is_empty() {
        let (file, meta) = files.remove(0);
        tracing::trace!(file_name = %file.path().display(), size = meta.size(), "Deleting file");
        tokio::fs::remove_file(file.path())
            .instrument(tracing::info_span!("Deleting file", file_name = %file.path().display(), size = meta.size()))
            .await?;
        total_size_bytes -= meta.size();
        total_files -= 1;
    }

    metrics::gauge!(crate::monitoring::ATTACHMENT_CACHE_DISK_USAGE_BYTES)
        .set(total_size_bytes as f64);
    metrics::gauge!(crate::monitoring::ATTACHMENT_CACHE_FILES).set(total_files as f64);

    Ok(())
}

fn generate_dirname(channel_id: ChannelId, msg_id: MessageId) -> String {
    format!("{}-{}", channel_id, msg_id)
}
