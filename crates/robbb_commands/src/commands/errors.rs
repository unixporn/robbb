use thiserror::Error;

#[derive(Error, Debug)]
#[error("{}", .0)]
pub struct UserErr(String);

impl UserErr {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Extension trait for both Option and Result that adds [UserErr] related context methods
pub trait OptionExt<T> {
    fn user_error(self, s: &str) -> Result<T, UserErr>;
}
impl<T, E: Into<anyhow::Error>> OptionExt<T> for Result<T, E> {
    fn user_error(self, s: &str) -> Result<T, UserErr> {
        self.map_err(|_| UserErr(s.to_string()))
    }
}

impl<T> OptionExt<T> for Option<T> {
    fn user_error(self, s: &str) -> Result<T, UserErr> {
        self.ok_or_else(|| UserErr(s.to_string()))
    }
}

/// Extension trait for Result that adds [UserErr] related context methods
pub trait ResultExt<T, E> {
    fn with_user_error(self, f: impl FnOnce(E) -> String) -> Result<T, UserErr>;
}

impl<T, E: Into<anyhow::Error>> ResultExt<T, E> for Result<T, E> {
    fn with_user_error(self, f: impl FnOnce(E) -> String) -> Result<T, UserErr> {
        self.map_err(|e| UserErr(f(e)))
    }
}
