use std::{fmt::Display, sync::Arc};

use anyhow::{Context, Result};
use chrono::Utc;

use extend::ext;
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client,
    model::{
        channel::Message,
        guild::Emoji,
        id::{ChannelId, GuildId},
        prelude::User,
    },
    utils::Colour,
};

use crate::{db::Db, Config, UPEmotes};

#[ext(pub)]
#[async_trait]
impl client::Context {
    async fn get_config(&self) -> Arc<Config> {
        self.data.read().await.get::<Config>().unwrap().clone()
    }
    async fn get_db(&self) -> Arc<Db> {
        self.data.read().await.get::<Db>().unwrap().clone()
    }
    async fn get_config_and_db(&self) -> (Arc<Config>, Arc<Db>) {
        let data = self.data.read().await;
        (
            data.get::<Config>().unwrap().clone(),
            data.get::<Db>().unwrap().clone(),
        )
    }

    async fn get_up_emotes(&self) -> Option<Arc<UPEmotes>> {
        self.data.read().await.get::<UPEmotes>().cloned()
    }

    async fn get_random_stare(&self) -> Option<Emoji> {
        self.get_up_emotes().await?.random_stare()
    }
}

#[ext(pub)]
impl User {
    fn name_with_disc_and_id(&self) -> String {
        format!("{}#{}({})", self.name, self.discriminator, self.id)
    }
}

#[ext(pub)]
#[async_trait]
impl GuildId {
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
        let pensibe = ctx
            .get_up_emotes()
            .await
            .map(|x| format!("{}", x.pensibe.clone()));
        self.reply_embed(&ctx, |e| {
            e.description(format!("{}{}", s, pensibe.unwrap_or_default()));
            e.color(0xfb4934);
        })
        .await
    }

    async fn reply_success(
        &self,
        ctx: &client::Context,
        s: impl Display + Send + Sync + 'static,
    ) -> Result<Message> {
        let poggers = ctx
            .get_up_emotes()
            .await
            .map(|x| format!("{}", x.poggers.clone()));

        self.reply_embed(&ctx, |e| {
            e.description(format!("{}{}", s, poggers.unwrap_or_default()));
            e.color(0xb8bb26);
        })
        .await
    }

    async fn reply_success_mod_action(
        &self,
        ctx: &client::Context,
        s: impl Display + Send + Sync + 'static,
    ) -> Result<Message> {
        let police = ctx
            .get_up_emotes()
            .await
            .map(|x| format!("{}", x.police.clone()));

        self.reply_embed(&ctx, |e| {
            e.description(format!("{}{}", s, police.unwrap_or_default()));
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
    let stare = ctx.get_random_stare().await;

    move |e: &mut CreateEmbed| {
        e.timestamp(&Utc::now());
        e.footer(|f| {
            if let Some(emoji) = stare {
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
