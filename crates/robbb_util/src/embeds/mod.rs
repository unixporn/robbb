use crate::{
    extensions::{ClientContextExt, PoiseContextExt},
    prelude::Ctx,
    UpEmotes,
};

use chrono::Utc;
use serenity::{
    builder::{CreateEmbed, CreateEmbedFooter},
    client,
};

pub mod paginated_embeds;
pub use paginated_embeds::*;

fn make_base_embed(emotes: Option<&UpEmotes>) -> CreateEmbed {
    CreateEmbed::default().timestamp(Utc::now()).footer({
        let mut f = CreateEmbedFooter::new("\u{200b}");
        if let Some(stare) = emotes.and_then(|x| x.random_stare()) {
            f = f.icon_url(stare.url());
        }
        f
    })
}

pub fn base_embed(ctx: &Ctx<'_>) -> CreateEmbed {
    make_base_embed(ctx.get_up_emotes().as_deref())
}

pub async fn base_embed_ctx(ctx: &client::Context) -> CreateEmbed {
    make_base_embed(ctx.get_up_emotes().await.as_deref())
}

pub async fn make_success_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let up_emotes = ctx.get_up_emotes().await;
    let emote = up_emotes.as_ref().map(|x| format!(" {}", x.poggers.clone()));
    make_base_embed(up_emotes.as_deref())
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xb8bb26u32)
}

pub async fn make_success_mod_action_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let up_emotes = ctx.get_up_emotes().await;
    let emote = up_emotes.as_ref().map(|x| format!(" {}", x.police.clone()));
    make_base_embed(up_emotes.as_deref())
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xb8bb26u32)
}

pub async fn make_error_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let up_emotes = ctx.get_up_emotes().await;
    let emote = up_emotes.as_ref().map(|x| format!(" {}", x.pensibe.clone()));
    make_base_embed(up_emotes.as_deref())
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xfb4934u32)
}
