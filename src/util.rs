use anyhow::*;
use std::env;

#[macro_export]
macro_rules! return_on_err {
    ($name:ident = $value:expr, $code:expr) => {
        match $value {
            Ok(x) => x,
            Err($name) => {
                $code;
                return;
            }
        }
    };
}

#[macro_export]
macro_rules! log_return_on_err {
    ($value:expr) => {
        return_on_err!(name = $value, eprintln!("{:?}", name))
    };
}

#[macro_export]
macro_rules! log_return_on_err_async {
    ($($value:tt)*) => {
        let result = async { $($value)* }.await;
        return_on_err!(name = result, eprintln!("{:?}", name))
    };
}

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

#[macro_export]
macro_rules! error_out {
    ($err:expr) => {
        Err($err)?;
    };
}

pub fn log_error_value<T, E: std::fmt::Debug>(result: Result<T, E>) {
    if let Err(e) = result {
        eprintln!("{:?}", e);
    }
}
pub fn required_env_var(key: &str) -> Result<String> {
    Ok(env::var(key).with_context(|| format!("Missing environment variable {}", key))?)
}

pub fn parse_required_env_var<E: Into<anyhow::Error>, T: std::str::FromStr<Err = E>>(
    key: &str,
) -> Result<T> {
    Ok(required_env_var(key)?
        .parse()
        .map_err(|e: E| anyhow!(e))
        .with_context(|| format!("Failed to parse env-var {}", key))?)
}

pub fn format_date(date: chrono::DateTime<chrono::Utc>) -> String {
    chrono_humanize::HumanTime::from(date).to_text_en(
        chrono_humanize::Accuracy::Rough,
        chrono_humanize::Tense::Past,
    )
}

pub fn format_count(num: i32) -> String {
    match num {
        1 => "1st".to_string(),
        2 => "2nd".to_string(),
        3 => "3rd".to_string(),
        _ => format!("{}th", num),
    }
}

pub fn validate_url(value: &str) -> bool {
    url::Url::parse(value)
        .map(|url| !url.scheme().is_empty() || url.host().is_some())
        .unwrap_or(false)
}
