use thiserror::Error;

// TODORW this is a placeholder, obviously
#[derive(Debug, Clone)]
pub struct CommandOptions {
    usage: Option<&'static str>,
}

#[derive(Error, Debug)]
pub enum UserErr {
    #[error("Couldn't find any user with that name")]
    MentionedUserNotFound,

    #[error("Usage: {0}")]
    InvalidUsage(&'static str),

    #[error("{0}")]
    Other(String),
}

impl UserErr {
    pub fn invalid_usage(opts: &CommandOptions) -> Self {
        Self::InvalidUsage(
            opts.usage
                .unwrap_or("RTFM, this is not how you use this command"),
        )
    }
    pub fn other(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

/// Extension trait for both Option and Result that adds [UserErr] related context methods
pub trait OptionExt<T> {
    fn invalid_usage(self, opts: &CommandOptions) -> Result<T, UserErr>;
    fn user_error(self, s: &str) -> Result<T, UserErr>;
}
impl<T, E: Into<anyhow::Error>> OptionExt<T> for Result<T, E> {
    fn invalid_usage(self, opts: &CommandOptions) -> Result<T, UserErr> {
        self.map_err(|_| UserErr::InvalidUsage(opts.usage.unwrap_or("")))
    }
    fn user_error(self, s: &str) -> Result<T, UserErr> {
        self.map_err(|_| UserErr::Other(s.to_string()))
    }
}

impl<T> OptionExt<T> for Option<T> {
    fn invalid_usage(self, opts: &CommandOptions) -> Result<T, UserErr> {
        self.ok_or_else(|| UserErr::InvalidUsage(opts.usage.unwrap_or("")))
    }

    fn user_error(self, s: &str) -> Result<T, UserErr> {
        self.ok_or_else(|| UserErr::Other(s.to_string()))
    }
}

/// Extension trait for Result that adds [UserErr] related context methods
pub trait ResultExt<T, E> {
    fn with_user_error(self, f: impl FnOnce(E) -> String) -> Result<T, UserErr>;
}

impl<T, E: Into<anyhow::Error>> ResultExt<T, E> for Result<T, E> {
    fn with_user_error(self, f: impl FnOnce(E) -> String) -> Result<T, UserErr> {
        self.map_err(|e| UserErr::Other(f(e)))
    }
}
