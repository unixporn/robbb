use crate::{extensions::ClientContextExt, UpEmotes};

use chrono::Utc;
use serenity::{builder::CreateEmbed, client};

pub mod paginated_embeds;
pub use paginated_embeds::*;

pub async fn make_create_embed(
    ctx: &client::Context,
    build: impl FnOnce(&mut CreateEmbed) -> &mut CreateEmbed,
) -> CreateEmbed {
    let stare = ctx
        .data
        .read()
        .await
        .get::<UpEmotes>()
        .and_then(|x| x.random_stare());

    let mut e = CreateEmbed::default();

    e.timestamp(Utc::now());
    e.footer(|f| {
        if let Some(emoji) = stare {
            f.icon_url(emoji.url());
        }
        f.text("\u{200b}")
    });

    build(&mut e);
    e
}

pub async fn make_success_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let emote = ctx
        .get_up_emotes()
        .await
        .as_ref()
        .map(|x| format!(" {}", x.poggers.clone()));

    let mut e = CreateEmbed::default();
    e.description(format!("{}{}", text, emote.unwrap_or_default()));
    e.color(0xb8bb26u32);
    e
}

pub async fn make_success_mod_action_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let emote = ctx
        .get_up_emotes()
        .await
        .as_ref()
        .map(|x| format!(" {}", x.police.clone()));

    let mut e = CreateEmbed::default();
    e.description(format!("{}{}", text, emote.unwrap_or_default()));
    e.color(0xb8bb26u32);
    e
}

pub async fn make_error_embed(ctx: &client::Context, text: &str) -> CreateEmbed {
    let emote = ctx
        .get_up_emotes()
        .await
        .as_ref()
        .map(|x| format!(" {}", x.pensibe.clone()));

    let mut e = CreateEmbed::default();
    e.description(format!("{}{}", text, emote.unwrap_or_default()));
    e.color(0xfb4934u32);
    e
}
