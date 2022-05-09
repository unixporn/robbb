#![allow(clippy::needless_borrow)]

use poise::serenity_prelude::GatewayIntents;
use robbb_commands::{checks, commands};
use robbb_db::Db;
use robbb_util::{
    config::Config,
    extensions::PoiseContextExt,
    log_error,
    prelude::{self, Ctx},
    util, UpEmotes, UserData,
};
use std::{ops::DerefMut, sync::Arc};
use tracing::Level;
use tracing_futures::Instrument;

pub mod attachment_logging;
pub mod events;
mod logging;

use crate::{events::handle_event, logging::*};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
                let up_emotes = match robbb_util::load_up_emotes(&ctx, config.guild).await {
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
            command_check: Some(|ctx| {
                Box::pin(async move {
                    println!("checking...");
                    Ok(checks::check_channel_allows_commands(ctx.clone()).await?
                        && checks::check_is_not_muted(ctx.clone()).await?)
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
        .await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(
    command_name = %ctx.command().name,
    msg.author = %ctx.author(),
    msg.id = %ctx.id(),
    msg.channel_id = %ctx.channel_id(),
))]
async fn before(ctx: Ctx<'_>) -> bool {
    tracing_honeycomb::register_dist_tracing_root(tracing_honeycomb::TraceId::new(), None).unwrap();
    let span = tracing::Span::current();
    ctx.set_invocation_data(span).await;

    let content = match ctx {
        poise::Context::Application(_) => "<slash command>".to_string(),
        poise::Context::Prefix(prefix) => prefix.msg.content.to_string(),
    };
    tracing::info!(
        command_name = ctx.command().name,
        msg.content = %content,
        msg.author = %ctx.author(),
        msg.id = %ctx.id(),
        msg.channel_id = %ctx.channel_id(),
        "command '{}' invoked by '{}'",
        ctx.command().name,
        ctx.author().tag()
    );
    true
}

fn framework_error_context<'a, 'b>(
    error: &'a poise::FrameworkError<'b, UserData, anyhow::Error>,
) -> Option<Ctx<'b>> {
    use poise::FrameworkError::*;
    match error {
        Command { ctx, .. }
        | ArgumentParse { ctx, .. }
        | CooldownHit { ctx, .. }
        | MissingBotPermissions { ctx, .. }
        | MissingUserPermissions { ctx, .. }
        | NotAnOwner { ctx }
        | GuildOnly { ctx }
        | DmOnly { ctx }
        | NsfwOnly { ctx }
        | CommandCheckFailed { ctx, .. } => Some(ctx.clone()),
        DynamicPrefix { .. } | Setup { .. } | CommandStructureMismatch { .. } | Listener { .. } => {
            None
        }
    }
}

/// Handler passed to poise
async fn on_error(error: poise::FrameworkError<'_, UserData, anyhow::Error>) {
    let ctx = framework_error_context(&error);
    let span: Option<tracing::Span> = if let Some(ctx) = ctx {
        let span = ctx.invocation_data::<tracing::Span>().await;
        span.map(|mut s| s.deref_mut().clone())
    } else {
        None
    };

    if let Some(span) = span {
        handle_poise_error(error).instrument(span).await;
    } else {
        handle_poise_error(error).await;
    }
}

async fn handle_poise_error(error: poise::FrameworkError<'_, UserData, prelude::Error>) {
    use poise::FrameworkError::*;
    match error {
        Command { error, ctx } => {
            handle_command_error(ctx, error).await;
        }
        Setup { error } => {
            tracing::error!(error = %error, "Error during setup: {}", error)
        }
        Listener {
            error,
            event,
            ctx: _,
            framework: _,
        } => {
            tracing::error!(event = ?event, error = %error, "Error in event listener: {}", error);
        }
        ArgumentParse { input, ctx, .. } => {
            log_error!(
                ctx.say_error(format!("Malformed value \"{}\"", input.unwrap_or_default()))
                    .await
            );
        }
        CommandStructureMismatch { description, ctx } => {
            log_error!(
                poise::Context::Application(ctx)
                    .say_error("Something went wrong")
                    .await
            );
            tracing::error!(error="CommandStructureMismach", error.description=%description, "Error in command structure: {}", description);
        }
        CooldownHit {
            remaining_cooldown,
            ctx,
        } => log_error!(
            ctx.say_error(format!(
                "You're doing this too much. Try again {}",
                util::format_date_ago(util::time_after_duration(remaining_cooldown))
            ))
            .await
        ),
        MissingBotPermissions {
            missing_permissions,
            ctx,
        } => {
            log_error!(
                ctx.say_error(format!(
                    "It seems like I am lacking the {} permission",
                    missing_permissions
                ))
                .await
            );
            tracing::error!(
                error = "Missing permissions",
                "Bot missing permissions: {}",
                missing_permissions
            )
        }
        MissingUserPermissions {
            missing_permissions,
            ctx,
        } => {
            log_error!(ctx.say_error("Missing permissions").await);
            tracing::error!(
                error = "User missing permissions",
                error.missing_permissions = ?missing_permissions,
                "User missing permissions: {:?}",
                missing_permissions
            )
        }
        NotAnOwner { ctx } => {
            log_error!(ctx.say_error("You need to be an owner to do this").await);
        }
        GuildOnly { ctx } => {
            log_error!(ctx.say_error("This can only be ran in a server").await);
        }
        DmOnly { ctx } => {
            log_error!(ctx.say_error("This can only be used in DMs").await);
        }
        NsfwOnly { ctx } => {
            log_error!(
                ctx.say_error("This can only be used in NSFW channels")
                    .await
            );
        }
        CommandCheckFailed { error, ctx } => {
            if let Some(error) = error {
                log_error!(
                    ctx.say_error("Something went wrong while checking your permissions")
                        .await
                );
                tracing::error!(
                    error = %error,
                    command_name = %ctx.command().name,
                    "Error while running command check: {}", error
                );
            } else {
                log_error!(ctx.say_error("Insufficient permissions").await);
            }
        }
        DynamicPrefix { error } => {
            tracing::error!(error = %error, "Error in dynamic prefix");
        }
    }
}

async fn handle_command_error(ctx: Ctx<'_>, err: prelude::Error) {
    match err.downcast_ref::<commands::UserErr>() {
        Some(err) => match err {
            commands::UserErr::MentionedUserNotFound => {
                let _ = ctx.say_error("No user found with that name").await;
            }
            commands::UserErr::Other(issue) => {
                let _ = ctx.say_error(format!("Error: {}", issue)).await;
            }
        },
        None => match err.downcast::<serenity::Error>() {
            Ok(err) => {
                //let err = *err;
                tracing::warn!(
                    error.command_name = %ctx.command().name,
                    error.message = %err,
                    "Serenity error [handling {}]: {} ({:?})",
                    ctx.command().name,
                    &err,
                    &err
                );
                match err {
                    serenity::Error::Http(err) => {
                        if let serenity::http::error::Error::UnsuccessfulRequest(res) = *err {
                            if res.status_code == serenity::http::StatusCode::NOT_FOUND
                                && res.error.message.to_lowercase().contains("unknown user")
                            {
                                let _ = ctx.say_error("User not found").await;
                            } else {
                                let _ = ctx.say_error("Something went wrong").await;
                            }
                        }
                    }
                    serenity::Error::Model(err) => {
                        let _ = ctx.say_error(format!("{}", err)).await;
                    }
                    _ => {
                        let _ = ctx.say_error("Something went wrong").await;
                    }
                }
            }
            Err(err) => {
                let _ = ctx.say_error("Something went wrong").await;
                tracing::warn!(
                    error.command_name = %ctx.command().name,
                    error.message = %err,
                    "Internal error [handling {}]: {} ({:#?})",
                    ctx.command().name,
                    &err,
                    &err
                );
            }
        },
    }
}
