use super::*;
use reqwest::multipart;

pub async fn message(ctx: client::Context, msg: Message) -> Result<()> {
    let config = ctx.data.read().await.get::<Config>().unwrap().clone();
    if msg.author.bot {
        return Ok(());
    }
    if msg.channel_id == config.channel_showcase {
        handle_showcase_post(ctx, msg)
            .await
            .context("Failed to handle showcase post")
    } else if msg.channel_id == config.channel_feedback {
        handle_feedback_post(ctx, msg)
            .await
            .context("Failed to handle feedback post")
    } else if msg.attachments.len() > 0 {
        message_txt(ctx, msg).await
    } else {
        Ok(())
    }
}

async fn handle_showcase_post(ctx: client::Context, msg: Message) -> Result<()> {
    if msg.attachments.is_empty() {
        msg.delete(&ctx)
            .await
            .context("Failed to delete invalid showcase submission")?;
        msg.author.direct_message(&ctx, |f| {
                f.content(indoc!("
                    Your showcase submission was detected to be invalid. If you wanna comment on a rice, use the #ricing-theming channel.
                    If this is a mistake, contact the moderators or open an issue on https://github.com/unixporn/trup
                "))
            }).await.context("Failed to send DM about invalid showcase submission")?;
    } else {
        msg.react(&ctx, ReactionType::Unicode("❤️".to_string()))
            .await
            .context("Error reacting to showcase submission with ❤️")?;
    }
    Ok(())
}

async fn handle_feedback_post(ctx: client::Context, msg: Message) -> Result<()> {
    tokio::try_join!(
        msg.react(&ctx, ReactionType::Unicode("👍".to_string())),
        msg.react(&ctx, ReactionType::Unicode("👎".to_string())),
    )
    .context("Error reacting to feedback submission with 👍")?;
    Ok(())
}

async fn message_txt(ctx: client::Context, msg: Message) -> Result<()> {
    let txt = msg.attachments.iter().find(|a| a.filename == "message.txt");
    if txt.is_none() {
        return Ok(());
    }

    let form = multipart::Form::new().text("url", txt.unwrap().url.clone());
    let code = reqwest::Client::builder()
        .https_only(true)
        .build()?
        .post("https://0x0.st")
        .multipart(form)
        .send()
        .await?;
    if code.status() != 200 {
        return Err(anyhow::anyhow!(format!(
            "0x0.st returned an error uploading the `message.txt` from {} ({}): \n{}",
            msg.author.name,
            msg.link(),
            code.text().await?
        )));
    }
    let text = code.text().await?;
    msg.reply_embed(&ctx, |m| {
        m.title(format!("{}", text));
        m.footer(|f| f.text(format!("message.txt of {}", msg.author.name)));
        m.color(serenity::utils::Color::from_rgb(3, 192, 60));
    })
    .await?;
    Ok(())
}
