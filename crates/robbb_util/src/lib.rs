pub mod cdn_hack;
pub mod collect_interaction;
pub mod config;
pub mod embeds;
pub mod extensions;
pub mod prelude;
pub mod util;

use std::{collections::HashMap, sync::Arc, time::Instant};

use dashmap::DashMap;
use eyre::ContextCompat as _;
use poise::serenity_prelude::{ChannelId, Emoji, GuildId, UserId};
use rand::{prelude::IteratorRandom, rngs::ThreadRng};
use robbb_db::Db;
use serenity::{all::EmojiId, client, prelude::TypeMapKey};

#[derive(Debug, Clone)]
pub struct UpEmotes {
    pub pensibe: Emoji,
    pub police: Emoji,
    pub poggers: Emoji,
    pub stares: Vec<Emoji>,
    pub all_emoji: HashMap<EmojiId, Emoji>,
}
impl UpEmotes {
    pub fn random_stare(&self) -> Option<Emoji> {
        let mut rng = ThreadRng::default();
        self.stares.iter().choose(&mut rng).cloned()
    }

    pub fn from_emojis(all_emoji: Vec<Emoji>) -> eyre::Result<Self> {
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
            stares: all_emoji.iter().filter(|x| x.name.starts_with("stare")).cloned().collect(),
            all_emoji: all_emoji.into_iter().map(|x| (x.id, x)).collect(),
        })
    }
}

#[tracing::instrument(skip_all)]
pub async fn load_up_emotes(ctx: &client::Context, guild: GuildId) -> eyre::Result<UpEmotes> {
    let all_emoji = guild.emojis(&ctx).await?;
    UpEmotes::from_emojis(all_emoji)
}

impl TypeMapKey for UpEmotes {
    type Value = Arc<UpEmotes>;
}

/// Short-lived cache of who deleted a message, populated reactively from
/// `GUILD_AUDIT_LOG_ENTRY_CREATE` gateway events so that `message_delete`
/// handling can look up the deleter without polling the audit-log REST endpoint.
///
/// Our approach here is that we store a map of
/// `(message-channel, message-author)` to `(deletor, deletion-timestamp)`.
/// Whenever a message gets deleted, we can then look up the last deletion of messages from that user in that channel,
/// and compare the timestamps. If the timestamp is close enough,
/// we assume that the event we got relates to the audit-log entry we found.
///
/// This isn't perfect, and MAY yield false positives, but should be close enough.
pub struct DeletionAuditCache {
    inner: DashMap<(ChannelId, UserId), (UserId, Instant)>,
}

const DELETION_CACHE_TTL_SECS: u64 = 60;

impl DeletionAuditCache {
    pub fn new() -> Self {
        Self { inner: DashMap::new() }
    }

    /// Record that `deleter_id` deleted a message from `target_user_id` in `channel_id`.
    pub fn insert(&self, channel_id: ChannelId, target_user_id: UserId, deleter_id: UserId) {
        let now = Instant::now();
        // Opportunistically evict stale entries on every insert.
        self.inner.retain(|_, (_, ts)| now.duration_since(*ts).as_secs() < DELETION_CACHE_TTL_SECS);
        self.inner.insert((channel_id, target_user_id), (deleter_id, now));
    }

    /// Look up who deleted a message from `target_user_id` in `channel_id`, if known.
    pub fn get(&self, channel_id: ChannelId, target_user_id: UserId) -> Option<UserId> {
        let now = Instant::now();
        self.inner
            .get(&(channel_id, target_user_id))
            .filter(|entry| now.duration_since(entry.value().1).as_secs() < DELETION_CACHE_TTL_SECS)
            .map(|entry| entry.value().0)
    }
}

impl Default for DeletionAuditCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeMapKey for DeletionAuditCache {
    type Value = Arc<DeletionAuditCache>;
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub config: Arc<config::Config>,
    pub db: Arc<Db>,
    pub up_emotes: Arc<arc_swap::ArcSwapOption<UpEmotes>>,
}
