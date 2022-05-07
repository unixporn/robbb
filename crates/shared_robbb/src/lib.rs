pub mod config;
pub mod embeds;
pub mod extensions;
pub mod prelude;
pub mod util;

use std::sync::Arc;

use anyhow::Context;
pub use config::*;
use db::Db;
pub use db_robbb as db;
pub use extensions::*;
use poise::serenity_prelude::{Emoji, GuildId, TypeMapKey};
use rand::prelude::IteratorRandom;
use serenity::client;

#[derive(Debug, Clone)]
pub struct UpEmotes {
    pub pensibe: Emoji,
    pub police: Emoji,
    pub poggers: Emoji,
    pub stares: Vec<Emoji>,
}
impl UpEmotes {
    pub fn random_stare(&self) -> Option<Emoji> {
        let mut rng = rand::thread_rng();
        self.stares.iter().choose(&mut rng).cloned()
    }
}

pub async fn load_up_emotes(ctx: &client::Context, guild: GuildId) -> anyhow::Result<UpEmotes> {
    let all_emoji = guild.emojis(&ctx).await?;
    Ok(UpEmotes {
        pensibe: all_emoji
            .iter()
            .find(|x| x.name == "pensibe")
            .context("no pensibe emote found")?
            .clone(),
        police: all_emoji
            .iter()
            .find(|x| x.name == "police")
            .context("no police emote found")?
            .clone(),
        poggers: all_emoji
            .iter()
            .find(|x| x.name == "poggersphisch")
            .context("no police poggers found")?
            .clone(),
        stares: all_emoji
            .into_iter()
            .filter(|x| x.name.starts_with("stare"))
            .collect(),
    })
}

impl TypeMapKey for UpEmotes {
    type Value = Arc<UpEmotes>;
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub config: Arc<Config>,
    pub db: Arc<Db>,
    pub up_emotes: Option<Arc<UpEmotes>>,
}
