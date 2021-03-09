use anyhow::*;
use std::env;

/// Run code in a try-block, logging any errors that occur.
#[macro_export]
macro_rules! log_errors {
    ($($code:tt)*) => {
        let result: ::anyhow::Result<()> = try { $($code)* };
        match result {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{:?}", err);
            }
        }
    };
}

/// return with an error value immediately.
#[macro_export]
macro_rules! abort_with {
    ($err:expr) => {
        return Err($err.into());
    };
}

pub fn log_error_value<T, E: std::fmt::Debug>(result: Result<T, E>) {
    if let Err(e) = result {
        eprintln!("{:?}", e);
    }
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
    chrono_humanize::HumanTime::from(date).to_text_en(
        chrono_humanize::Accuracy::Rough,
        chrono_humanize::Tense::Past,
    )
}

/// Format a number into the 1st, 2nd, 3rd, 4th,... format
pub fn format_count(num: i32) -> String {
    match num {
        1 => "1st".to_string(),
        2 => "2nd".to_string(),
        3 => "3rd".to_string(),
        _ => format!("{}th", num),
    }
}

/// Validate that a string is a valid URL.
pub fn validate_url(value: &str) -> bool {
    url::Url::parse(value)
        .map(|url| !url.scheme().is_empty() || url.host().is_some())
        .unwrap_or(false)
}
