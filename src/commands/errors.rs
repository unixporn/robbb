use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserErr {
    #[error("Couldn't find any user with that name")]
    MentionedUserNotFound,

    #[error("{0}")]
    Other(String),
}

impl UserErr {
    pub fn other(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

/// Extension trait for both Option and Result that adds [UserErr] related context methods
pub trait OptionExt<T> {
    fn user_error(self, s: &str) -> Result<T, UserErr>;
}
impl<T, E: Into<anyhow::Error>> OptionExt<T> for Result<T, E> {
    fn user_error(self, s: &str) -> Result<T, UserErr> {
        self.map_err(|_| UserErr::Other(s.to_string()))
    }
}

impl<T> OptionExt<T> for Option<T> {
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
