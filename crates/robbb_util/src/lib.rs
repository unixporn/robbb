pub mod collect_interaction;
pub mod config;
pub mod embeds;
pub mod extensions;
pub mod modal;
pub mod prelude;
pub mod util;

use std::sync::Arc;

use anyhow::Context;
use poise::serenity_prelude::{Emoji, GuildId, TypeMapKey};
use rand::prelude::IteratorRandom;
use robbb_db::Db;
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

#[tracing::instrument(skip_all)]
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
            .context("no poggers emote found")?
            .clone(),
        stares: all_emoji.into_iter().filter(|x| x.name.starts_with("stare")).collect(),
    })
}

impl TypeMapKey for UpEmotes {
    type Value = Arc<UpEmotes>;
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub config: Arc<config::Config>,
    pub db: Arc<Db>,
    pub up_emotes: Arc<parking_lot::RwLock<Option<Arc<UpEmotes>>>>,
}
