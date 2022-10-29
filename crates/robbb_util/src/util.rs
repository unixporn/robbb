use anyhow::{anyhow, Context, Result};
use poise::serenity_prelude::GuildId;
use serenity::{
    client,
    model::{id::ChannelId, misc::EmojiIdentifier},
};
use std::env;

/// return with an error value immediately.
#[macro_export]
macro_rules! abort_with {
    ($err:literal) => {
        return Err(UserErr::other($err).into())
    };
    ($err:expr) => {
        return Err($err.into())
    };
}

/// If the result of the given code is an error, log it nicely. Otherwise just ignore the value.
#[macro_export]
macro_rules! log_error {
    ($e:expr) => {
        if let Err(e) = $e {
            let e = anyhow::anyhow!(e);
            tracing::error!(
                error.message = %&e,
                error.root_cause = %e.root_cause(),
                "{:?}",
                e
            );
        }
    };
    ($context:expr, $e:expr $(,)?) => {
        if let Err(e) = $e {
            let e = ::anyhow::anyhow!(e).context($context);
            tracing::error!(
                error.message = %&e,
                error.root_cause = %e.root_cause(),
                "{:?}",
                e
            );
        }
    };
}

pub fn ellipsis_text(text: &str, max_len: usize) -> String {
    if text.len() + 3 > max_len {
        let mut cutoff = max_len - 3;
        while !text.is_char_boundary(cutoff) {
            cutoff -= 1;
        }
        format!("{}...", text.split_at(cutoff).0)
    } else {
        text.to_string()
    }
}

pub fn thread_title_from_text(text: &str) -> Result<String> {
    let title = text.lines().find(|x| !x.trim().is_empty()).context("Text was empty")?;

    Ok(ellipsis_text(title, 96))
}

/// Get an environment variable, returning an Err with a
/// nice error message mentioning the missing variable in case the value is not found.
pub fn required_env_var(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("Missing environment variable {}", key))
}

/// like [required_env_var], but also uses FromStr to parse the value.
pub fn parse_required_env_var<E: Into<anyhow::Error>, T: std::str::FromStr<Err = E>>(
    key: &str,
) -> Result<T> {
    required_env_var(key)?
        .parse()
        .map_err(|e: E| anyhow!(e))
        .with_context(|| format!("Failed to parse env-var {}", key))
}

pub fn time_after_duration(duration: std::time::Duration) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
        .checked_add_signed(chrono::Duration::from_std(duration).unwrap())
        .unwrap_or_else(chrono::Utc::now)
}

/// Format a date into a discord relative-time timestamp.
pub fn format_date_ago(date: chrono::DateTime<chrono::Utc>) -> String {
    format!("<t:{}:R>", date.timestamp())
}

/// Format a date into the time difference between it and another date in a plain text relative-time format.
pub fn format_date_before_plaintext(
    a: chrono::DateTime<chrono::Utc>,
    b: chrono::DateTime<chrono::Utc>,
) -> String {
    let actual_date = a.checked_add_signed(chrono::Utc::now().signed_duration_since(b)).unwrap();
    let formatted = chrono_humanize::HumanTime::from(actual_date)
        .to_text_en(chrono_humanize::Accuracy::Rough, chrono_humanize::Tense::Past);
    // lmao
    if formatted == "now ago" {
        "now".to_string()
    } else {
        formatted
    }
}

/// Format a date into a discord absolute-time timestamp.
pub fn format_date(date: chrono::DateTime<chrono::Utc>) -> String {
    format!("<t:{}>", date.timestamp())
}

/// Format a date, showing both the concrete date and the "n days ago"-format.
pub fn format_date_detailed(date: chrono::DateTime<chrono::Utc>) -> String {
    format!("{} ({})", format_date(date), format_date_ago(date))
}

/// Format a number into an ordinal, like 1st, 2nd, 3rd
pub fn format_count(num: i32) -> String {
    let last_digits = num % 100;

    if (11..=13).contains(&last_digits) {
        format!("{}th", num)
    } else {
        match last_digits % 10 {
            1 => format!("{}st", num),
            2 => format!("{}nd", num),
            3 => format!("{}rd", num),
            _ => format!("{}th", num),
        }
    }
}

/// Find all emojis in a String
#[tracing::instrument(skip_all)]
pub fn find_emojis(value: impl AsRef<str>) -> Vec<EmojiIdentifier> {
    lazy_static::lazy_static! {
        static ref FIND_EMOJI : regex::Regex = regex::Regex::new(r"<a?:[0-9a-zA-Z_]{2,32}:[0-9]{18,}>").unwrap();
    }
    FIND_EMOJI
        .find_iter(value.as_ref())
        .filter_map(|x| serenity::utils::parse_emoji(x.as_str()))
        .collect()
}

/// Validate that a string is a valid URL.
pub fn validate_url(value: &str) -> bool {
    url::Url::parse(value)
        .map(|url| !url.scheme().is_empty() && url.host().is_some() && url.domain().is_some())
        .unwrap_or(false)
}

pub fn pluralize(s: &str) -> String {
    if let Some(word) = s.strip_suffix("ys") {
        format!("{}ies", word)
    } else {
        s.to_string()
    }
}

/// Parse a string that is surrounded by backticks, removing said backticks.
/// Returns a [UserErr::Other] in case the string is not properly surrounded in `
pub fn parse_backticked_string(s: &str) -> Option<&str> {
    s.strip_prefix('`').and_then(|x| x.strip_suffix('`'))
}

/// Determine if a file is an image based on the file extension
pub fn is_image_file(s: &str) -> bool {
    match s.split('.').last() {
        Some(ext) => matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "webp"),
        None => false,
    }
}

/// Return the bot version, as read from the VERSION environment variable at build time.
pub fn bot_version() -> &'static str {
    option_env!("VERSION").unwrap_or("<no version>")
}

pub async fn channel_name(ctx: &client::Context, channel_id: ChannelId) -> Result<String> {
    let channel = channel_id
        .to_channel(&ctx)
        .await
        .context("Failed to get channel object for channel id")?
        .guild()
        .context("Failed to get guild channel for channel object")?;
    Ok(channel.name().to_string())
}

/// Discord snowflakes are just fancy timestamps, kinda.
/// This function converts a time into a discord snowflake that may be used to link to a point in chat without requiring a specific message.
///
/// See [here](https://discord.com/developers/docs/reference#snowflake-ids-in-pagination-generating-a-snowflake-id-from-a-timestamp-example)
pub fn time_to_discord_snowflake(time: chrono::DateTime<chrono::Utc>) -> i64 {
    const DISCORD_EPOCH: i64 = 1420070400000;
    (time.timestamp_millis() - DISCORD_EPOCH) << 22
}

pub fn generate_message_link(
    guild_id: Option<GuildId>,
    channel_id: ChannelId,
    message_id: i64,
) -> String {
    match guild_id {
        Some(guild_id) => {
            format!("https://discord.com/channels/{}/{}/{}", guild_id, channel_id, message_id)
        }
        None => format!("https://discord.com/channels/@me/{}/{}", channel_id, message_id),
    }
}
