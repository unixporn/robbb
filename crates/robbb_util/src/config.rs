use std::{collections::HashSet, path::PathBuf, sync::Arc};

use poise::serenity_prelude::{ChannelId, CreateEmbed, GuildId, RoleId};
use serenity::{all::UserId, client, prelude::TypeMapKey};

use crate::{
    extensions::GuildIdExt,
    log_error,
    util::{parse_required_env_var, required_env_var},
};

#[derive(Debug)]
pub struct Config {
    pub discord_token: String,

    pub owners: HashSet<UserId>,

    pub guild: GuildId,
    pub role_mod: RoleId,
    pub role_helper: RoleId,
    pub role_mute: RoleId,
    pub role_htm: RoleId,
    pub roles_color: Vec<RoleId>,

    pub category_mod_private: ChannelId,
    pub channel_announcements: ChannelId,
    pub channel_rules: ChannelId,
    pub channel_showcase: ChannelId,
    pub channel_feedback: ChannelId,
    pub channel_modlog: ChannelId,
    pub channel_mod_bot_stuff: ChannelId,
    pub channel_auto_mod: ChannelId,
    pub channel_bot_messages: ChannelId,
    pub channel_bot_traffic: ChannelId,
    pub channel_tech_support: ChannelId,
    pub channel_mod_polls: ChannelId,
    pub channel_attachment_dump: Option<ChannelId>,
    pub channel_fake_cdn: ChannelId,

    pub attachment_cache_path: PathBuf,
    pub attachment_cache_max_size: usize,

    pub time_started: chrono::DateTime<chrono::Utc>,
}

impl Config {
    pub fn from_environment() -> anyhow::Result<Self> {
        Ok(Config {
            discord_token: required_env_var("TOKEN")?,
            owners: required_env_var("OWNERS")?
                .split(',')
                .map(|x| Ok(x.trim().parse()?))
                .collect::<anyhow::Result<_>>()?,

            guild: GuildId::new(parse_required_env_var("GUILD")?),
            role_mod: RoleId::new(parse_required_env_var("ROLE_MOD")?),
            role_helper: RoleId::new(parse_required_env_var("ROLE_HELPER")?),
            role_mute: RoleId::new(parse_required_env_var("ROLE_MUTE")?),
            role_htm: RoleId::new(parse_required_env_var("ROLE_HTM")?),
            roles_color: required_env_var("ROLES_COLOR")?
                .split(',')
                .map(|x| Ok(x.trim().parse()?))
                .collect::<anyhow::Result<_>>()?,
            category_mod_private: ChannelId::new(parse_required_env_var("CATEGORY_MOD_PRIVATE")?),
            channel_announcements: ChannelId::new(parse_required_env_var("CHANNEL_ANNOUNCEMENTS")?),
            channel_rules: ChannelId::new(parse_required_env_var("CHANNEL_RULES")?),
            channel_showcase: ChannelId::new(parse_required_env_var("CHANNEL_SHOWCASE")?),
            channel_feedback: ChannelId::new(parse_required_env_var("CHANNEL_FEEDBACK")?),
            channel_modlog: ChannelId::new(parse_required_env_var("CHANNEL_MODLOG")?),
            channel_auto_mod: ChannelId::new(parse_required_env_var("CHANNEL_AUTO_MOD")?),
            channel_mod_bot_stuff: ChannelId::new(parse_required_env_var("CHANNEL_MOD_BOT_STUFF")?),
            channel_bot_messages: ChannelId::new(parse_required_env_var("CHANNEL_BOT_MESSAGES")?),
            channel_bot_traffic: ChannelId::new(parse_required_env_var("CHANNEL_BOT_TRAFFIC")?),
            channel_tech_support: ChannelId::new(parse_required_env_var("CHANNEL_TECH_SUPPORT")?),
            channel_mod_polls: ChannelId::new(parse_required_env_var("CHANNEL_MOD_POLLS")?),
            channel_attachment_dump: parse_required_env_var("CHANNEL_ATTACHMENT_DUMP")
                .map(ChannelId::new)
                .ok(),
            channel_fake_cdn: ChannelId::new(parse_required_env_var("CHANNEL_FAKE_CDN")?),
            attachment_cache_path: parse_required_env_var("ATTACHMENT_CACHE_PATH")?,
            attachment_cache_max_size: parse_required_env_var("ATTACHMENT_CACHE_MAX_SIZE")?,
            time_started: chrono::Utc::now(),
        })
    }

    pub async fn log_bot_action<F>(&self, ctx: &client::Context, build_embed: F)
    where
        F: FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    {
        let result = self.guild.send_embed(ctx, self.channel_modlog, build_embed).await;

        log_error!(result);
    }
    pub async fn log_automod_action<F>(&self, ctx: &client::Context, build_embed: F)
    where
        F: FnOnce(CreateEmbed) -> CreateEmbed + Send + Sync,
    {
        let result = self.guild.send_embed(ctx, self.channel_auto_mod, build_embed).await;
        log_error!(result);
    }
}

impl TypeMapKey for Config {
    type Value = Arc<Config>;
}
