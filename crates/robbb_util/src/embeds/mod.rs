use crate::{UpEmotes, UserData};

use chrono::Utc;
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

pub mod paginated_embeds;
pub use paginated_embeds::*;

fn make_base_embed<'a>(emotes: Option<&UpEmotes>) -> CreateEmbed<'a> {
    CreateEmbed::default().timestamp(Utc::now()).footer({
        let mut f = CreateEmbedFooter::new("\u{200b}");
        if let Some(stare) = emotes.and_then(|x| x.random_stare()) {
            f = f.icon_url(stare.url());
        }
        f
    })
}

pub fn base_embed<'a>(user_data: impl AsRef<UserData>) -> CreateEmbed<'a> {
    make_base_embed(user_data.as_ref().up_emotes().as_deref())
}

pub fn make_success_embed<'a>(user_data: impl AsRef<UserData>, text: &str) -> CreateEmbed<'a> {
    let up_emotes = user_data.as_ref().up_emotes();
    let emote = up_emotes.as_ref().map(|x| format!(" {}", x.poggers.clone()));
    make_base_embed(up_emotes.as_deref())
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xb8bb26u32)
}

pub fn make_success_mod_action_embed<'a>(
    user_data: impl AsRef<UserData>,
    text: &str,
) -> CreateEmbed<'a> {
    let up_emotes = user_data.as_ref().up_emotes();
    let emote = up_emotes.as_ref().map(|x| format!(" {}", x.police.clone()));
    make_base_embed(up_emotes.as_deref())
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xb8bb26u32)
}

pub fn make_error_embed<'a>(user_data: impl AsRef<UserData>, text: &str) -> CreateEmbed<'a> {
    let up_emotes = user_data.as_ref().up_emotes();
    let emote = up_emotes.as_ref().map(|x| format!(" {}", x.pensibe.clone()));
    make_base_embed(up_emotes.as_deref())
        .description(format!("{}{}", text, emote.unwrap_or_default()))
        .color(0xfb4934u32)
}
