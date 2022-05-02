#![allow(clippy::needless_borrow)]
use anyhow::Context;
use db::Db;
use poise::serenity_prelude::{GatewayIntents, TypeMapKey};
use prelude::Ctx;
use rand::prelude::IteratorRandom;
use serenity::{client, model::prelude::*};
use std::sync::Arc;
use tracing::Level;

pub mod attachment_logging;
pub mod checks;
pub mod commands;
pub mod config;
pub mod db;
pub mod embeds;
pub mod events;
pub mod extensions;
mod logging;
pub mod modlog;
pub mod prelude;
pub mod util;
use crate::{events::handle_event, logging::*};
pub use config::*;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct UpEmotes {
    pensibe: Emoji,
    police: Emoji,
    poggers: Emoji,
    stares: Vec<Emoji>,
}
impl UpEmotes {
    pub fn random_stare(&self) -> Option<Emoji> {
        let mut rng = rand::thread_rng();
        self.stares.iter().choose(&mut rng).cloned()
    }
}

async fn load_up_emotes(ctx: &client::Context, guild: GuildId) -> anyhow::Result<UpEmotes> {
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
    config: Arc<Config>,
    db: Arc<Db>,
    up_emotes: Option<Arc<UpEmotes>>,
}

#[tokio::main]
async fn main() {
    let honeycomb_api_key = std::env::var("HONEYCOMB_API_KEY").ok();

    init_tracing(honeycomb_api_key.clone());
    if let Some(honeycomb_api_key) = honeycomb_api_key {
        send_honeycomb_deploy_marker(&honeycomb_api_key).await;
    }

    let span = tracing::span!(Level::DEBUG, "main");
    let _enter = span.enter();

    init_cpu_logging().await;

    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();

    let config = Config::from_environment().expect("Failed to load experiment");

    let db = Db::new().await.expect("Failed to initialize database");
    db.run_migrations().await.unwrap();
    db.remove_forbidden_highlights().await.unwrap();

    poise::Framework::build()
        .token(config.discord_token.to_string())
        .client_settings(|client| client.cache_settings(|cache| cache.max_messages(500)))
        .user_data_setup(|ctx, _ready, _framework| {
            Box::pin(async move {
                let config = Arc::new(config);
                let db = Arc::new(db);
                ctx.data.write().await.insert::<Config>(config.clone());
                ctx.data.write().await.insert::<Db>(db.clone());
                let up_emotes = match load_up_emotes(&ctx, config.guild).await {
                    Ok(emotes) => {
                        let emotes = Arc::new(emotes.clone());
                        ctx.data.write().await.insert::<UpEmotes>(emotes.clone());
                        Some(emotes)
                    }
                    Err(err) => {
                        tracing::warn!("Error loading emotes: {}", err);
                        None
                    }
                };

                Ok(UserData {
                    config,
                    db,
                    up_emotes,
                })
            })
        })
        .intents(GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT)
        .options(poise::FrameworkOptions {
            commands: commands::all_commands(),
            on_error: |err| Box::pin(on_error(err)),
            pre_command: |ctx| {
                Box::pin(async move {
                    println!("Executing command {}...", ctx.command().qualified_name);
                    before(ctx).await;
                })
            },
            post_command: |ctx| {
                Box::pin(async move {
                    println!("Executed command {}!", ctx.command().qualified_name);
                })
            },
            /// Every command invocation must pass this check to continue execution
            command_check: Some(|_ctx| {
                Box::pin(async move {
                    println!("checking...");
                    Ok(true)
                })
            }),

            listener: |ctx, event, framework, data| {
                Box::pin(async move {
                    println!("Got an event in listener: {:?}", event.name());
                    handle_event(ctx, event, framework, data.clone()).await;
                    Ok(())
                })
            },
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                edit_tracker: Some(poise::EditTracker::for_timespan(
                    std::time::Duration::from_secs(10),
                )),
                execute_untracked_edits: true,
                execute_self_messages: false,
                case_insensitive_commands: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
        .await
        .unwrap();
}

async fn before(ctx: Ctx<'_>) -> bool {
    tracing::debug!(
        command_name = ctx.command().name,
        //msg.content = %msg.content, // TODORW
        msg.author = %ctx.author(),
        msg.id = %ctx.id(),
        msg.channel_id = %ctx.channel_id(),
        "command '{}' invoked by '{}'",
        ctx.command().name,
        ctx.author().tag()
    );
    true
}

async fn on_error(error: poise::FrameworkError<'_, UserData, Error>) {
    eprintln!("on_error: {:?}", error);
}

// TODORW
//#[tracing::instrument(skip_all, fields(%command_name, %msg.content, %msg.channel_id, %error))]
//async fn dispatch_error_hook(
//ctx: &client::Context,
//msg: &Message,
//error: DispatchError,
//command_name: &str,
//) {
//// Log dispatch errors that should be logged
//match &error {
//DispatchError::CheckFailed(required, Reason::Log(log))
//| DispatchError::CheckFailed(required, Reason::UserAndLog { user: _, log }) => {
//tracing::warn!("Check for {} failed with: {}", required, log);
//}
//_ => {}
//};

//let _ = msg
//.reply_error(&ctx, display_dispatch_error(command_name, error))
//.await;
//}

// TODORW
//fn display_dispatch_error(command_name: &str, err: DispatchError) -> String {
//match err {
//DispatchError::CheckFailed(_required, reason) => match reason {
//Reason::User(reason)
//| Reason::UserAndLog {
//user: reason,
//log: _,
//} => reason,
//_ => "You're not allowed to use this command".to_string(),
//},
//DispatchError::Ratelimited(_info) => "Hit a rate-limit".to_string(),
//DispatchError::CommandDisabled => format!("Command {} is disabled", command_name),
//DispatchError::BlockedUser => "User not allowed to use bot".to_string(),
//DispatchError::BlockedGuild => "Guild is blocked by bot".to_string(),
//DispatchError::BlockedChannel => "Channel is blocked by bot".to_string(),
//DispatchError::OnlyForDM => "Command may only be used in DMs".to_string(),
//DispatchError::OnlyForGuilds => "Command may only be used in a server".to_string(),
//DispatchError::OnlyForOwners => "Command may only be used by owners".to_string(),
//DispatchError::LackingRole => "Missing a required role".to_string(),
//DispatchError::LackingPermissions(flags) => format!(
//"User is missing permissions - required permission number is {}",
//flags
//),
//DispatchError::NotEnoughArguments { min, given } => format!(
//"Not enough arguments provided - got {} but needs {}",
//given, min
//),
//DispatchError::TooManyArguments { max, given } => format!(
//"Too many arguments provided - got {} but can only handle {}",
//given, max
//),
//_ => {
//tracing::error!("Unhandled dispatch error: {:?}", err);
//"Failed to run command".to_string()
//}
//}
//}

// TODORW
//async fn after(ctx: &client::Context, msg: &Message, command_name: &str, result: CommandResult) {
//if let Err(err) = result {
//match err.downcast_ref::<UserErr>() {
//Some(err) => match err {
//UserErr::MentionedUserNotFound => {
//let _ = msg.reply_error(&ctx, "No user found with that name").await;
//}
//UserErr::InvalidUsage(usage) => {
//let _ = msg.reply_error(&ctx, format!("Usage: {}", usage)).await;
//}
//UserErr::Other(issue) => {
//let _ = msg.reply_error(&ctx, format!("Error: {}", issue)).await;
//}
//},
//None => match err.downcast::<serenity::Error>() {
//Ok(err) => {
//let err = *err;
//tracing::warn!(
//error.command_name = %command_name,
//error.message = %err,
//"Serenity error [handling {}]: {} ({:?})",
//command_name,
//&err,
//&err
//);
//match err {
//serenity::Error::Http(err) => {
//if let serenity::http::error::Error::UnsuccessfulRequest(res) = *err {
//if res.status_code == serenity::http::StatusCode::NOT_FOUND
//&& res.error.message.to_lowercase().contains("unknown user")
//{
//let _ = msg.reply_error(&ctx, "User not found").await;
//} else {
//let _ = msg.reply_error(&ctx, "Something went wrong").await;
//}
//}
//}
//serenity::Error::Model(err) => {
//let _ = msg.reply_error(&ctx, err).await;
//}
//_ => {
//let _ = msg.reply_error(&ctx, "Something went wrong").await;
//}
//}
//}
//Err(err) => {
//let _ = msg.reply_error(&ctx, "Something went wrong").await;
//tracing::warn!(
//error.command_name = %command_name,
//error.message = %err,
//"Internal error [handling {}]: {} ({:#?})",
//command_name,
//&err,
//&err
//);
//}
//},
//}
//}
//}
