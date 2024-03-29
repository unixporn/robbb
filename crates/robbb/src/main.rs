#![allow(clippy::needless_borrow)]

use anyhow::Context;
use poise::{serenity_prelude::GatewayIntents, CommandInteractionType};
use pyroscope::PyroscopeAgent;
use pyroscope_pprofrs::{pprof_backend, PprofConfig};
use robbb_commands::{checks, commands};
use robbb_db::Db;

use robbb_util::{config::Config, extensions::ChannelIdExt, prelude::Ctx, UserData};
use serenity::all::OnlineStatus;
use std::sync::Arc;

pub mod attachment_logging;
mod error_handling;
pub mod events;
mod logging;

use crate::logging::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let honeycomb_api_key = std::env::var("HONEYCOMB_API_KEY").ok();

    let pyroscope_url = std::env::var("PYROSCOPE_URL").ok();
    let pyroscope_project = std::env::var("PYROSCOPE_PROJECT").ok();
    let pyroscope_user = std::env::var("PYROSCOPE_USER").ok();
    let pyroscope_password = std::env::var("PYROSCOPE_PASSWORD").ok();

    init_tracing();
    if let Some(honeycomb_api_key) = honeycomb_api_key {
        send_honeycomb_deploy_marker(&honeycomb_api_key).await;
    }

    let pyroscope_running = if let Some((url, project)) = pyroscope_url.zip(pyroscope_project) {
        tracing::info!("Enabling pyroscope profiling");
        let mut agent_builder = PyroscopeAgent::builder(url, project);
        if let Some((username, password)) = pyroscope_user.zip(pyroscope_password) {
            agent_builder = agent_builder.basic_auth(username, password);
        }
        let agent =
            agent_builder.backend(pprof_backend(PprofConfig::new().sample_rate(100))).build()?;
        Some(agent.start()?)
    } else {
        None
    };

    let mut client = setup_discord_client().await?;

    let bot_version = robbb_util::util::BotVersion::get();
    tracing::info!(
        version.profile = %bot_version.profile,
        version.commit_hash = %bot_version.commit_hash,
        version.commit_msg = %bot_version.commit_msg,
        "Starting discord client"
    );
    client.start().await?;

    if let Some(pyroscope_running) = pyroscope_running {
        let pyroscope_ready = pyroscope_running.stop().context("Failed to stop pyroscope agent")?;
        pyroscope_ready.shutdown();
    }

    Ok(())
}

#[tracing::instrument]
async fn setup_discord_client() -> anyhow::Result<serenity::Client> {
    tracing::info!("Starting setup");
    let config = Config::from_environment().expect("Failed to load experiment");
    tracing::info!(config = ?config, "Loaded configuration");

    let db = Db::new().await.expect("Failed to initialize database");
    db.run_migrations().await.expect("Failed to run DB migrations");
    tracing::info!("Ran DB migrations");
    db.remove_forbidden_highlights().await.expect("Failed to remove forbidden highlights from DB");
    tracing::info!("Removed forbidden highlights from DB");

    let framework_options = poise::FrameworkOptions {
        commands: commands::all_commands(),
        on_error: |err| Box::pin(error_handling::on_error(err)),
        skip_checks_for_owners: true,
        pre_command: |ctx| Box::pin(pre_command(ctx)),
        owners: config.owners.clone(),
        command_check: Some(|ctx| {
            Box::pin(async move {
                Ok(is_autocomplete_interaction(&ctx)
                    || (checks::check_channel_allows_commands(ctx).await?
                        && checks::check_is_not_muted(ctx).await?))
            })
        }),
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("!".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                std::time::Duration::from_secs(10),
            ))),
            execute_untracked_edits: true,
            execute_self_messages: false,
            case_insensitive_commands: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let gateway_intents = GatewayIntents::all();

    let config = Arc::new(config);
    let db = Arc::new(db);

    let event_handler = Arc::new(events::Handler::new(
        framework_options,
        UserData {
            config: config.clone(),
            db: db.clone(),
            up_emotes: Arc::new(parking_lot::RwLock::new(None)),
        },
    ));

    tracing::info!("Initialized event handler");

    let client = serenity::Client::builder(&config.discord_token, gateway_intents)
        .activity(serenity::gateway::ActivityData::listening("/help"))
        .status(OnlineStatus::Online)
        .event_handler_arc(event_handler.clone())
        .cache_settings({
            let mut settings = serenity::cache::Settings::default();
            settings.max_messages = 500;
            settings
        })
        .await
        .expect("Error creating client");

    {
        let mut client_data = client.data.write().await;
        client_data.insert::<Config>(config);
        client_data.insert::<Db>(db);
    }

    event_handler.set_shard_manager(client.shard_manager.clone());
    tracing::info!("Initialized client");
    Ok(client)
}

async fn pre_command(ctx: Ctx<'_>) {
    let content = match ctx {
        poise::Context::Application(_) => ctx.invocation_string(),
        poise::Context::Prefix(prefix) => prefix.msg.content.to_string(),
    };
    let channel_name = ctx.channel_id().name_cached_or_fallback(&ctx.cache());

    let span = tracing::Span::current();
    span.record("command_name", ctx.command().qualified_name.as_str());
    span.record("invocation", ctx.invocation_string());
    span.record("msg.content", content.as_str());
    span.record("msg.author", ctx.author().tag().as_str());
    span.record("msg.id", ctx.id());
    span.record("msg.channel_id", ctx.channel_id().get());
    span.record("msg.channel", channel_name.as_str());

    tracing::info!(
        command_name = ctx.command().qualified_name.as_str(),
        invocation = ctx.invocation_string(),
        msg.content = %content,
        msg.author = %ctx.author().tag(),
        msg.author_id = %ctx.author().id,
        msg.id = %ctx.id(),
        msg.channel = %channel_name,
        msg.channel_id = %ctx.channel_id(),
        "{} invoked by {}",
        ctx.command().name,
        ctx.author().tag()
    );
}

fn is_autocomplete_interaction(ctx: &Ctx<'_>) -> bool {
    match ctx {
        poise::Context::Application(ctx) => {
            matches!(ctx.interaction_type, CommandInteractionType::Autocomplete)
        }
        _ => false,
    }
}
