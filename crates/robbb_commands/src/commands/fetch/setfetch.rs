use anyhow::Context;
use chrono::Utc;
use poise::serenity_prelude::{Attachment, AttachmentType, ChannelId, Http};

use super::*;
use std::{borrow::Cow, collections::HashMap};

const SETFETCH_USAGE: &str = indoc::indoc!("
    Run this: 
    `curl -s https://raw.githubusercontent.com/unixporn/robbb/master/fetcher.sh | sh`
    and follow the instructions. It's recommended that you download and read the script before running it, 
    as piping curl to sh isn't always the safest practice. (<https://blog.dijit.sh/don-t-pipe-curl-to-bash>) 

    **NOTE**: use `/setfetch update` to manually update your fetch (including the image!).
    **NOTE**: /git, /dotfiles, and /description are different commands"
);

/// Set your fetch data
#[poise::command(
    slash_command,
    guild_only,
    rename = "setfetch",
    subcommands("set_fetch_script", "set_fetch_update", "set_fetch_clear")
)]
pub async fn set_fetch(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

/// Use our custom fetch script to fill in your entire fetch automatically!
#[poise::command(slash_command, guild_only, rename = "script")]
pub async fn set_fetch_script(ctx: Ctx<'_>) -> Res<()> {
    ctx.send_embed_full(true, |e| {
        e.description(SETFETCH_USAGE);
    })
    .await?;
    Ok(())
}

/// Update your fetch data
#[poise::command(slash_command, guild_only, rename = "update")]
#[allow(clippy::too_many_arguments)]
pub async fn set_fetch_update(
    ctx: Ctx<'_>,
    #[description = "Image"] image: Option<Attachment>,
    #[description = "Distro"] distro: Option<String>,
    #[description = "Kernel"] kernel: Option<String>,
    #[description = "Terminal"] terminal: Option<String>,
    #[description = "Editor"] editor: Option<String>,
    #[description = "Shell"] shell: Option<String>,
    #[description = "De_wm"] de_wm: Option<String>,
    #[description = "Bar"] bar: Option<String>,
    #[description = "Resolution"] resolution: Option<String>,
    #[description = "Display Protocol"] display_protocol: Option<String>,
    #[description = "GTK3 Theme"] gtk3_theme: Option<String>,
    #[description = "GTK icon theme"] gtk_icon_theme: Option<String>,
    #[description = "CPU"] cpu: Option<String>,
    #[description = "GPU"] gpu: Option<String>,
    #[description = "Description"] description: Option<String>,
    #[description = "Dotfiles"] dotfiles: Option<String>,
    #[description = "Git"] git: Option<String>,
    #[description = "Memory"] memory: Option<String>,
) -> Res<()> {
    let config = ctx.get_config();
    let image = match (image, config.channel_attachment_dump) {
        (Some(image), Some(dump_channel)) => {
            ctx.defer().await?;
            Some(dump_attachment(&ctx.serenity_context(), dump_channel, image).await?)
        }
        (None, _) => None,
        (Some(image), None) => {
            tracing::warn!("Setfetch called with image, but there is no attachment dump configured. Discord will delete the image after a while. Please configure an attachment_dump channel.");
            Some(image.url)
        }
    };

    let memory = if let Some(memory) = memory {
        Some(
            byte_unit::Byte::from_str(memory)
                .user_error("Malformed value provided for Memory")?
                .get_bytes()
                .to_string(),
        )
    } else {
        None
    };

    let data = maplit::hashmap! {
        FetchField::Image => image,
        FetchField::Distro => distro,
        FetchField::Kernel => kernel,
        FetchField::Terminal => terminal,
        FetchField::Editor => editor,
        FetchField::Shell => shell,
        FetchField::DEWM => de_wm,
        FetchField::Bar => bar,
        FetchField::Resolution => resolution,
        FetchField::DisplayProtocol => display_protocol,
        FetchField::GTK3 => gtk3_theme,
        FetchField::Icons => gtk_icon_theme,
        FetchField::CPU => cpu,
        FetchField::GPU => gpu,
        FetchField::Dotfiles => dotfiles,
        FetchField::Description => description,
        FetchField::Git => git,
        FetchField::Memory => memory,
    };
    let info = data.into_iter().filter_map(|(k, v)| Some((k, v?))).collect();
    let db = ctx.get_db();
    db.update_fetch(ctx.author().id, info).await?;
    ctx.say_success("Successfully updated your fetch data!").await?;

    Ok(())
}

/// Clear your fetch data
#[poise::command(slash_command, guild_only, prefix_command, rename = "clear")]
pub async fn set_fetch_clear(
    ctx: Ctx<'_>,
    #[description = "Field you want to clear"] field: Option<FetchField>,
) -> Res<()> {
    let db = ctx.get_db();

    if let Some(field) = field {
        let old_fetch = db.get_fetch(ctx.author().id).await?;
        if let Some(mut fetch) = old_fetch {
            fetch.info.remove(&field);
            db.set_fetch(ctx.author().id, fetch.info, Some(Utc::now())).await?;
        }
        ctx.say_success(format!("Successfully cleared your {}", field)).await?;
    } else {
        db.set_fetch(ctx.author().id, HashMap::new(), Some(Utc::now())).await?;
        ctx.say_success("Successfully cleared your fetch data!").await?;
    }
    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn dump_attachment(
    http: impl AsRef<Http>,
    dump_channel: ChannelId,
    attachment: Attachment,
) -> anyhow::Result<String> {
    let file = attachment.download().await?;
    let message = dump_channel
        .send_files(
            http,
            [AttachmentType::Bytes { data: Cow::from(file), filename: attachment.filename }],
            |m| m,
        )
        .await?;
    Ok(message.attachments.first().context("No attachment in dump message, weird")?.url.to_string())
}
