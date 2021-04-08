use anyhow::*;
use chrono::Timelike;
use serenity::model::id::EmojiId;
use std::env;

/// return with an error value immediately.
#[macro_export]
macro_rules! abort_with {
    ($err:literal) => {
        return Err(UserErr::other($err).into());
    };
    ($err:expr) => {
        return Err($err.into());
    };
}

#[macro_export]
macro_rules! log_error {
    ($e:expr) => {
        if let Err(e) = $e {
            log::error!("{:?}", e);
        }
    };
    ($context:expr, $e:expr $(,)?) => {
        if let Err(e) = $e {
            log::error!("{:?}", ::anyhow::anyhow!(e).context($context));
        }
    };
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

/// Format a date into a normalized "2 days ago"-like format.
pub fn format_date_ago(date: chrono::DateTime<chrono::Utc>) -> String {
    let formatted = chrono_humanize::HumanTime::from(date).to_text_en(
        chrono_humanize::Accuracy::Rough,
        chrono_humanize::Tense::Past,
    );
    // lmao
    if formatted == "now ago" {
        "now".to_string()
    } else {
        formatted
    }
}

/// Format a date.
pub fn format_date(date: chrono::DateTime<chrono::Utc>) -> String {
    let date = date.with_nanosecond(0).unwrap_or(date);
    format!("{}", date)
}

/// Format a date, showing both the concrete date and the "n days ago"-format.
pub fn format_date_detailed(date: chrono::DateTime<chrono::Utc>) -> String {
    let date = date.with_nanosecond(0).unwrap_or(date);
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

///Find all emojis in a String
pub fn find_emojis(value: &String) -> Result<Vec<(String, EmojiId)>> {
    lazy_static::lazy_static! {
        static ref reg : regex::Regex = regex::Regex::new(r"<:\s(\w\w+?):\s([0-9]\w+?)>|<a:\s([a-z]\w+?):\s([0-9]\w+?)>").unwrap();
    }
    Ok(reg
        .captures_iter(value)
        .filter_map(|x| {
            x.get(1)
                .and_then(|y| Some((y.as_str().into(), EmojiId(x.get(2)?.as_str().parse().ok()?))))
                .or(x.get(3).and_then(|y| {
                    Some((y.as_str().into(), EmojiId(x.get(4)?.as_str().parse().ok()?)))
                }))
        })
        .collect())
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
