use std::fmt::Display;

use anyhow::{Context, Result};
use chrono::Utc;

use extend::ext;
use rand::prelude::IteratorRandom;
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client,
    model::{
        channel::Message,
        guild::{Emoji, Guild},
        id::{ChannelId, GuildId},
        prelude::User,
    },
    utils::Colour,
};

use crate::Config;

#[ext(pub)]
impl User {
    fn name_with_disc_and_id(&self) -> String {
        format!("{}#{}({})", self.name, self.discriminator, self.id)
    }
}

#[ext(pub)]
#[async_trait]
impl Guild {
    async fn random_stare_emoji(&self, ctx: &client::Context) -> Option<Emoji> {
        self.id.random_stare_emoji(&ctx).await
    }

    async fn send_embed<F>(
        &self,
        ctx: &client::Context,
        channel_id: ChannelId,
        build: F,
    ) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        self.id.send_embed(ctx, channel_id, build).await
    }
}

#[ext(pub)]
#[async_trait]
impl GuildId {
    async fn random_stare_emoji(&self, ctx: &client::Context) -> Option<Emoji> {
        self.emojis(&ctx)
            .await
            .map(|emoji| {
                let mut rng = rand::thread_rng();
                emoji
                    .into_iter()
                    .filter(|e| e.name.starts_with("stare"))
                    .choose(&mut rng)
            })
            .unwrap_or(None)
    }

    async fn send_embed<F>(
        &self,
        ctx: &client::Context,
        channel_id: ChannelId,
        build: F,
    ) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let build_basics = build_embed_builder(&ctx).await;
        Ok(channel_id
            .send_message(&ctx, |m| {
                m.embed(|e| {
                    build_basics(e);
                    build(e);
                    e
                })
            })
            .await
            .context("Failed to send embed message")?)
    }
}

#[ext(pub)]
#[async_trait]
impl Message {
    async fn reply_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let build_basics = build_embed_builder(&ctx).await;

        self.channel_id
            .send_message(&ctx, move |m| {
                m.allowed_mentions(|f| f.replied_user(false));
                m.reference_message(self);
                m.embed(move |e| {
                    build_basics(e);
                    build(e);
                    e
                })
            })
            .await
            .context("Failed to send embed")
    }

    async fn reply_error(
        &self,
        ctx: &client::Context,
        s: impl Display + Send + Sync + 'static,
    ) -> Result<Message> {
        self.reply_embed(&ctx, |e| {
            e.description(format!("{}", s));
            e.color(0xfb4934);
        })
        .await
    }

    async fn reply_success(
        &self,
        ctx: &client::Context,
        s: impl Display + Send + Sync + 'static,
    ) -> Result<Message> {
        self.reply_embed(&ctx, |e| {
            e.description(format!("{}", s));
            e.color(0xb8bb26);
        })
        .await
    }

    fn to_context_link(&self) -> String {
        format!("[(context)]({})", self.link())
    }
}

#[ext(pub)]
#[async_trait]
impl ChannelId {
    async fn send_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let build_basics = build_embed_builder(&ctx).await;
        Ok(self
            .send_message(&ctx, |m| {
                m.embed(|e| {
                    build_basics(e);
                    build(e);
                    e
                })
            })
            .await
            .context("Failed to send embed message")?)
    }
}

pub async fn build_embed_builder(ctx: &client::Context) -> impl FnOnce(&mut CreateEmbed) {
    let data = ctx.data.read().await;
    let config = data.get::<Config>().unwrap().clone();

    let guild = config.guild;
    let emoji = guild.random_stare_emoji(&ctx).await;

    move |e: &mut CreateEmbed| {
        e.timestamp(&Utc::now());
        e.footer(|f| {
            if let Some(emoji) = emoji {
                f.icon_url(emoji.url());
            }
            f.text("\u{200b}")
        });
    }
}

#[ext(pub)]
impl CreateEmbed {
    fn color_opt(&mut self, c: Option<impl Into<Colour>>) -> &mut CreateEmbed {
        if let Some(c) = c {
            self.color(c);
        }
        self
    }
}

#[ext(pub, name = StrExt)]
impl<T: AsRef<str>> T {
    fn split_once(&self, c: char) -> Option<(&str, &str)> {
        let s: &str = self.as_ref();
        let index = s.find(c)?;
        Some(s.split_at(index))
    }
}
