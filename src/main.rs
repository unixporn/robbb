#![feature(try_blocks)]
#![feature(label_break_value)]
use serenity::client::{self, Client};
use serenity::framework::standard::macros::hook;
use serenity::framework::standard::DispatchError;
use serenity::framework::standard::StandardFramework;
use serenity::http::CacheHttp;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{async_trait, client::bridge::gateway::GatewayIntents};
use std::sync::Arc;

use crate::util::*;
use anyhow::{anyhow, Context, Result};

pub mod checks;
pub mod commands;
pub mod events;
pub mod util;

use commands::*;

struct Config {
    pub discord_token: String,

    pub guild: GuildId,
    pub role_mod: RoleId,
    pub role_helper: RoleId,
    pub role_mute: RoleId,
    pub roles_color: Vec<RoleId>,

    pub category_mod_private: ChannelId,
    pub channel_showcase: ChannelId,
    pub channel_feedback: ChannelId,
    pub channel_modlog: ChannelId,
    pub channel_auto_mod: ChannelId,
    pub channel_bot_messages: ChannelId,
    pub channel_bot_traffic: ChannelId,
}

impl Config {
    fn from_environment() -> Result<Self> {
        Ok(Config {
            discord_token: required_env_var("TOKEN")?,
            guild: GuildId(parse_required_env_var("GUILD")?),
            role_mod: RoleId(parse_required_env_var("ROLE_MOD")?),
            role_helper: RoleId(parse_required_env_var("ROLE_HELPER")?),
            role_mute: RoleId(parse_required_env_var("ROLE_MUTE")?),
            roles_color: required_env_var("ROLES_COLOR")?
                .split(',')
                .map(|x| Ok(RoleId(x.trim().parse()?)))
                .collect::<Result<_>>()?,
            category_mod_private: ChannelId(parse_required_env_var("CATEGORY_MOD_PRIVATE")?),
            channel_showcase: ChannelId(parse_required_env_var("CHANNEL_SHOWCASE")?),
            channel_feedback: ChannelId(parse_required_env_var("CHANNEL_FEEDBACK")?),
            channel_modlog: ChannelId(parse_required_env_var("CHANNEL_MODLOG")?),
            channel_auto_mod: ChannelId(parse_required_env_var("CHANNEL_AUTO_MOD")?),
            channel_bot_messages: ChannelId(parse_required_env_var("CHANNEL_BOT_MESSAGES")?),
            channel_bot_traffic: ChannelId(parse_required_env_var("CHANNEL_BOT_TRAFFIC")?),
        })
    }
}

impl TypeMapKey for Config {
    type Value = Arc<Config>;
}

#[tokio::main]
async fn main() {
    let config = Config::from_environment().expect("Failed to load experiment");
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .on_dispatch_error(dispatch_error_hook)
        .group(&MODERATOR_GROUP)
        .group(&GENERAL_GROUP)
        .help(&MY_HELP);

    let mut client = Client::builder(&config.discord_token)
        .event_handler(events::Handler)
        .framework(framework)
        .intents(GatewayIntents::all())
        .await
        .expect("Error creating client");

    client.cache_and_http.cache.set_max_messages(500).await;

    {
        let mut data = client.data.write().await;
        data.insert::<Config>(Arc::new(config));
    };

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[hook]
async fn dispatch_error_hook(ctx: &client::Context, msg: &Message, error: DispatchError) {
    dbg!(&error);
}
