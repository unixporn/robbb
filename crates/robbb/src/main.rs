#![allow(clippy::needless_borrow)]

use poise::{serenity_prelude::GatewayIntents, CommandInteractionType};
use robbb_commands::{checks, commands};
use robbb_db::Db;

use robbb_util::{config::Config, prelude::Ctx, UserData};
use serenity::all::OnlineStatus;
use std::sync::Arc;
use tracing::Level;

pub mod attachment_logging;
mod error_handling;
pub mod events;
mod logging;

use crate::logging::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let honeycomb_api_key = std::env::var("HONEYCOMB_API_KEY").ok();

    init_tracing(honeycomb_api_key.clone());
    if let Some(honeycomb_api_key) = honeycomb_api_key {
        send_honeycomb_deploy_marker(&honeycomb_api_key).await;
    }

    let span = tracing::span!(Level::DEBUG, "main");
    let _enter = span.enter();

    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();

    let config = Config::from_environment().expect("Failed to load experiment");

    let db = Db::new().await.expect("Failed to initialize database");
    db.run_migrations().await.unwrap();
    db.remove_forbidden_highlights().await.unwrap();

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

    let mut client = serenity::Client::builder(&config.discord_token, gateway_intents)
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

    client.start().await?;

    Ok(())
}

async fn pre_command(ctx: Ctx<'_>) {
    let content = match ctx {
        poise::Context::Application(_) => ctx.invocation_string(),
        poise::Context::Prefix(prefix) => prefix.msg.content.to_string(),
    };
    let channel_name = ctx
        .channel_id()
        .to_channel_cached(ctx.cache())
        .and_then(|x| x.guild(&ctx))
        .map(|x| x.name.to_string())
        .unwrap_or_default();

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
