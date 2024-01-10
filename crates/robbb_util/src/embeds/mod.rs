use crate::{extensions::ClientContextExt, UpEmotes};

use chrono::Utc;
use poise::serenity_prelude::Context;
use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter},
    client,
};

pub mod paginated_embeds;
pub use paginated_embeds::*;

pub async fn base_embed(ctx: &Context) -> CreateEmbed {
    let stare = ctx.data.read().await.get::<UpEmotes>().and_then(|x| x.random_stare());

    CreateEmbed::default().timestamp(Utc::now()).footer({
        let mut f = CreateEmbedFooter::new("\u{200b}");
        if let Some(emoji) = stare {
            f = f.icon_url(emoji.url());
        }
        f
    })
}

pub async fn make_success_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let emote = ctx.get_up_emotes().await.as_ref().map(|x| format!(" {}", x.poggers.clone()));
    base_embed(ctx)
        .await
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xb8bb26u32)
}

pub async fn make_success_mod_action_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let emote = ctx.get_up_emotes().await.as_ref().map(|x| format!(" {}", x.police.clone()));
    base_embed(ctx)
        .await
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xb8bb26u32)
}

pub async fn make_error_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let emote = ctx.get_up_emotes().await.as_ref().map(|x| format!(" {}", x.pensibe.clone()));
    base_embed(ctx)
        .await
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xfb4934u32)
}
