pub use crate::UserData;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// Calling this Res is a temporary workaround until poise fixes the fact that it's macros rely on Result being std::result::Result...
pub type Res<T> = std::result::Result<T, Error>;

pub type Ctx<'a> = poise::Context<'a, UserData, Error>;
pub type AppCtx<'a> = poise::ApplicationContext<'a, UserData, Error>;
