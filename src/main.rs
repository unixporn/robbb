use crate::extensions::*;
#[allow(unused_imports)]
use db::Db;
use serenity::client::bridge::gateway::GatewayIntents;
#[allow(unused_imports)]
use serenity::client::{self, Client};
use serenity::framework::standard::DispatchError;
use serenity::framework::standard::{macros::hook, CommandResult, Reason};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{builder::CreateEmbed, framework::standard::StandardFramework};
use std::io::Write;
use std::{path::PathBuf, sync::Arc};

use crate::util::*;
use anyhow::Result;

pub mod attachment_logging;
pub mod checks;
pub mod commands;
pub mod db;
pub mod events;
pub mod extensions;
pub mod util;

use commands::*;

pub struct Config {
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

    pub attachment_cache_path: PathBuf,
    pub attachment_cache_max_size: usize,
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
            attachment_cache_path: parse_required_env_var("ATTACHMENT_CACHE_PATH")?,
            attachment_cache_max_size: parse_required_env_var("ATTACHMENT_CACHE_MAX_SIZE")?,
        })
    }

    async fn log_bot_action<F>(&self, ctx: &client::Context, build_embed: F)
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let result = self
            .guild
            .send_embed(&ctx, self.channel_modlog, build_embed)
            .await;
        log_error!(result);
    }
    async fn log_automod_action<F>(&self, ctx: &client::Context, build_embed: F)
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let result = self
            .guild
            .send_embed(&ctx, self.channel_auto_mod, build_embed)
            .await;
        log_error!(result);
    }

    #[allow(unused)]
    async fn is_mod(&self, ctx: &client::Context, user_id: UserId) -> Result<bool> {
        let user = user_id.to_user(&ctx).await?;
        Ok(user.has_role(&ctx, self.guild, self.role_mod).await?)
    }
}

impl TypeMapKey for Config {
    type Value = Arc<Config>;
}

#[tokio::main]
async fn main() {
    init_logger();

    let config = Config::from_environment().expect("Failed to load experiment");

    let db = Db::new().await.unwrap();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!").delimiters(vec![" ", "\n"]))
        .on_dispatch_error(dispatch_error_hook)
        .after(after)
        .group(&MODERATOR_GROUP)
        .group(&HELPERORMOD_GROUP)
        .group(&GENERAL_GROUP)
        .help(&help::MY_HELP);

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

        data.insert::<Db>(Arc::new(db));
    };

    if let Err(why) = client.start().await {
        log::error!("An error occurred while running the client: {:?}", why);
    }
}

#[hook]
async fn dispatch_error_hook(ctx: &client::Context, msg: &Message, error: DispatchError) {
    // Log dispatch errors that should be logged
    match &error {
        DispatchError::CheckFailed(required, Reason::Log(log))
        | DispatchError::CheckFailed(required, Reason::UserAndLog { user: _, log }) => {
            log::warn!("Check for {} failed with: {}", required, log);
        }
        _ => {}
    };

    let _ = msg.reply_error(&ctx, display_dispatch_error(error)).await;
}

fn display_dispatch_error(err: DispatchError) -> String {
    match err {
        DispatchError::CheckFailed(required, reason) => match reason {
            Reason::User(reason)
            | Reason::UserAndLog {
                user: reason,
                log: _,
            } => format!("{}\nRequires {}", reason, required),
            _ => "You're not allowed to use this command".to_string(),
        },
        DispatchError::Ratelimited(_info) => "Hit a rate-limit".to_string(),
        DispatchError::CommandDisabled(_) => "Command is disabled".to_string(),
        DispatchError::BlockedUser => "User not allowed to use bot".to_string(),
        DispatchError::BlockedGuild => "Guild is blocked by bot".to_string(),
        DispatchError::BlockedChannel => "Channel is blocked by bot".to_string(),
        DispatchError::OnlyForDM => "Command may only be used in DMs".to_string(),
        DispatchError::OnlyForGuilds => "Command may only be used in a server".to_string(),
        DispatchError::OnlyForOwners => "Command may only be used by owners".to_string(),
        DispatchError::LackingRole => "Missing a required role".to_string(),
        DispatchError::LackingPermissions(flags) => format!(
            "User is missing permissions - required permission number is {}",
            flags
        ),
        DispatchError::NotEnoughArguments { min, given } => format!(
            "Not enough arguments provided - got {} but needs {}",
            given, min
        ),
        DispatchError::TooManyArguments { max, given } => format!(
            "Too many arguments provided - got {} but can only handle {}",
            given, max
        ),
        _ => {
            log::warn!("Unhandled dispatch error: {:?}", err);
            "Failed to run command".to_string()
        }
    }
}

#[hook]
async fn after(ctx: &client::Context, msg: &Message, command_name: &str, result: CommandResult) {
    match result {
        Err(err) => match err.downcast_ref::<UserErr>() {
            Some(err) => match err {
                UserErr::MentionedUserNotFound => {
                    let _ = msg.reply_error(&ctx, "No user found with that name").await;
                }
                UserErr::InvalidUsage(usage) => {
                    let _ = msg.reply_error(&ctx, format!("Usage: {}", usage)).await;
                }
                UserErr::Other(issue) => {
                    let _ = msg.reply_error(&ctx, format!("Error: {}", issue)).await;
                }
            },
            None => match err.downcast::<serenity::Error>() {
                Ok(err) => {
                    let err = *err;
                    log::warn!(
                        "Serenity error [handling {}]: {} ({:?})",
                        command_name,
                        &err,
                        &err
                    );
                    match err {
                        serenity::Error::Http(err) => match *err {
                            serenity::http::error::Error::UnsuccessfulRequest(res) => {
                                if res.status_code == serenity::http::StatusCode::NOT_FOUND
                                    && res.error.message.to_lowercase().contains("unknown user")
                                {
                                    let _ = msg.reply_error(&ctx, "User not found").await;
                                } else {
                                    let _ = msg.reply_error(&ctx, "Something went wrong").await;
                                }
                            }
                            _ => {}
                        },
                        serenity::Error::Model(err) => {
                            let _ = msg.reply_error(&ctx, err).await;
                        }
                        _ => {
                            let _ = msg.reply_error(&ctx, "Something went wrong").await;
                        }
                    }
                }
                Err(err) => {
                    let _ = msg.reply_error(&ctx, "Something went wrong").await;
                    log::warn!(
                        "Internal error [handling {}]: {} ({:?})",
                        command_name,
                        &err,
                        &err
                    );
                }
            },
        },
        Ok(()) => {}
    }
}

fn init_logger() {
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder
        .format(|buf, r| {
            let ts = buf.timestamp();
            let level = buf.default_styled_level(r.level());
            let mut bold = buf.style();
            bold.set_bold(true);

            let module_or_file = if r.file().is_some() && r.file().unwrap().len() < 80 {
                format!(
                    "{}:{}",
                    r.file().unwrap_or_default(),
                    r.line().unwrap_or_default()
                )
            } else {
                format!("{}", r.module_path().unwrap_or_default())
            };

            writeln!(
                buf,
                "{} {} [{}] {} {}",
                ts,
                level,
                module_or_file,
                bold.value(">"),
                r.args()
            )
        })
        .filter_module("trup_rs", log::LevelFilter::Debug);

    if let Some(log_var) = std::env::var("RUST_LOG").ok() {
        builder.parse_filters(&log_var);
    }
    builder.init();
}
