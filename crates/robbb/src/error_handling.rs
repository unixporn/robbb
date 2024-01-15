use poise::{
    serenity_prelude::{MemberParseError, UserParseError},
    CreateReply, TooFewArguments, TooManyArguments,
};
use robbb_commands::commands;

use robbb_util::{
    extensions::PoiseContextExt,
    log_error,
    prelude::{self, Ctx},
    util, UserData,
};

/// Handler passed to poise
pub async fn on_error(error: poise::FrameworkError<'_, UserData, prelude::Error>) {
    use poise::FrameworkError::*;
    if let Some(ctx) = error.ctx() {
        tracing::error!(
            error.message = %error,
            error = ?error,
            command_name = %ctx.command().qualified_name,
            invocation = %ctx.invocation_string(),
            author.tag = %ctx.author().tag(),
            "Error occured in context, more details will follow"
        );
    }
    match error {
        Command { error, ctx, .. } => {
            handle_command_error(ctx, error).await;
        }

        CommandPanic { payload, ctx, .. } => {
            tracing::error!(
                error.message = %payload.unwrap_or_default(),
                command_name = ctx.command().qualified_name,
                command.code_name = ctx.command().source_code_name,
                invocation = %ctx.invocation_string(),
                "Command panicked"
            );
        }
        Setup { error, .. } => {
            tracing::error!(error.message = %error, "Error during setup: {}", error)
        }
        EventHandler { error, event, ctx: _, framework: _, .. } => {
            tracing::error!(event = ?event, error.message = %error, "Error in event listener: {}", error);
        }
        ArgumentParse { input, ctx, error, .. } => {
            log_error!(handle_argument_parse_error(ctx, error, input).await);
        }
        CommandStructureMismatch { description, ctx, .. } => {
            log_error!(poise::Context::Application(ctx).say_error("Something went wrong").await);
            tracing::error!(
                error.message = "CommandStructureMismach",
                error.description = %description,
                command.invocation = %ctx.invocation_string(),
                "Error in command structure: {description}"
            );
        }
        CooldownHit { remaining_cooldown, ctx, .. } => log_error!(
            ctx.say_error(format!(
                "You're doing this too much. Try again {}",
                util::format_date_ago(util::time_after_duration(remaining_cooldown))
            ))
            .await
        ),
        MissingBotPermissions { missing_permissions, ctx, .. } => {
            log_error!(
                ctx.say_error(format!(
                    "It seems like I am lacking the {missing_permissions} permission",
                ))
                .await
            );
            tracing::error!(
                error.message = "Missing permissions",
                command.name = ctx.command().qualified_name,
                command.invocation = %ctx.invocation_string(),
                "Bot missing permissions: {missing_permissions}",
            )
        }
        MissingUserPermissions { missing_permissions, ctx, .. } => {
            log_error!(ctx.say_error("Missing permissions").await);
            tracing::error!(
                error.message = "User missing permissions",
                error.missing_permissions = ?missing_permissions,
                command.author = ctx.author().tag(),
                command.invocation = %ctx.invocation_string(),
                "User missing permissions: {:?}",
                missing_permissions
            )
        }
        NotAnOwner { ctx, .. } => {
            log_error!(ctx.say_error("You need to be an owner to do this").await);
        }
        GuildOnly { ctx, .. } => {
            log_error!(ctx.say_error("This can only be ran in a server").await);
        }
        DmOnly { ctx, .. } => {
            log_error!(ctx.say_error("This can only be used in DMs").await);
        }
        NsfwOnly { ctx, .. } => {
            log_error!(ctx.say_error("This can only be used in NSFW channels").await);
        }
        CommandCheckFailed { error, ctx, .. } => {
            if let Some(error) = error {
                log_error!(
                    ctx.say_error("Something went wrong while checking your permissions").await
                );
                tracing::error!(
                    error.message = %error,
                    command.name = %ctx.command().qualified_name.as_str(),
                    command.invocation = %ctx.invocation_string(),
                    "Error while running command check: {}", error
                );
            } else if matches!(ctx, poise::Context::Application(_)) {
                log_error!(
                    ctx.send(
                        CreateReply::default().ephemeral(true).content("Insufficient permissions")
                    )
                    .await
                );
            }
        }
        DynamicPrefix { error, .. } => {
            tracing::error!(error.message = %error, "Error in dynamic prefix");
        }
        other => {
            if let Some(ctx) = other.ctx() {
                tracing::error!(
                    error.message = %other,
                    error = ?other,
                    command.author.tag = ctx.author().tag(),
                    command.name = ctx.command().qualified_name,
                    command.invocation = %ctx.invocation_string(),
                    "unhandled error received from poise"
                );
            } else {
                tracing::error!(error.message = %other, error = ?other, "unhandled error received from poise");
            }
        }
    }
}

async fn handle_argument_parse_error(
    ctx: Ctx<'_>,
    error: Box<dyn std::error::Error + Send + Sync>,
    input: Option<String>,
) -> anyhow::Result<()> {
    let msg = if error.downcast_ref::<humantime::DurationError>().is_some() {
        format!("'{}' is not a valid duration", input.unwrap_or_default())
    } else if error.downcast_ref::<UserParseError>().is_some() {
        format!("I couldn't find any user '{}'", input.unwrap_or_default())
    } else if error.downcast_ref::<MemberParseError>().is_some() {
        format!("I couldn't find any member '{}'", input.unwrap_or_default())
    } else if error.downcast_ref::<TooManyArguments>().is_some() {
        "Too many arguments".to_string()
    } else if error.downcast_ref::<TooFewArguments>().is_some() {
        "Too few arguments".to_string()
    } else if let Some(input) = input {
        format!("Malformed argument '{}'", input)
    } else {
        "Command used incorrectly".to_string()
    };
    ctx.say_error(msg).await?;
    Ok(())
}

async fn handle_command_error(ctx: Ctx<'_>, err: prelude::Error) {
    match err.downcast_ref::<commands::UserErr>() {
        Some(inner_err) => match inner_err {
            commands::UserErr::MentionedUserNotFound => {
                let _ = ctx.say_error("No user found with that name").await;
            }
            commands::UserErr::Other(issue) => {
                let _ = ctx.say_error(format!("Error: {}", issue)).await;
                tracing::info!(
                    user_error.message=%issue,
                    user_error.command_name = %ctx.command().qualified_name.as_str(),
                    user_error.invocation = %ctx.invocation_string(),
                    "User error"
                );
            }
        },
        None => match err.downcast_ref::<serenity::Error>() {
            Some(inner_err) => {
                tracing::warn!(
                    error.command_name = %ctx.command().qualified_name.as_str(),
                    error.invocation = %ctx.invocation_string(),
                    error.message = %err,
                    error.root_cause = %err.root_cause(),
                    "Serenity error [handling {}]: {} ({:?})",
                    ctx.command().name,
                    &err,
                    &inner_err
                );
                match inner_err {
                    serenity::Error::Http(err) => {
                        if let serenity::all::HttpError::UnsuccessfulRequest(res) = err {
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
            None => {
                let _ = ctx.say_error("Something went wrong").await;
                tracing::warn!(
                    error.command_name = %ctx.command().qualified_name.as_str(),
                    error.invocation = %ctx.invocation_string(),
                    error.message = %err,
                    error.root_cause = %err.root_cause(),
                    "Internal error [handling {}]: {} ({:#?})",
                    ctx.command().name,
                    &err,
                    &err
                );
            }
        },
    }
}
