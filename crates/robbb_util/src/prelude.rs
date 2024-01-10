use poise::serenity_prelude::{CreateEmbed};

pub use crate::UserData;

pub type Error = anyhow::Error; // Box<dyn std::error::Error + Send + Sync>;

/// Calling this Res is a temporary workaround until poise fixes the fact that it's macros rely on Result being std::result::Result...
pub type Res<T> = anyhow::Result<T>; // std::result::Result<T, Error>;

pub type Ctx<'a> = poise::Context<'a, UserData, Error>;
pub type AppCtx<'a> = poise::ApplicationContext<'a, UserData, Error>;
pub type PrefixCtx<'a> = poise::PrefixContext<'a, UserData, Error>;

//pub type BoxedCreateMessageBuilder = Box<
//    dyn for<'a, 'b> FnOnce(&'a mut CreateMessage<'b>) -> &'a mut CreateMessage<'b> + Send + Sync,
//>;
pub type BoxedCreateEmbedBuilder<'a> = Box<dyn FnOnce(&mut CreateEmbed) + Send + Sync + 'a>;
