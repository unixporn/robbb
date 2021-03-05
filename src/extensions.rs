use anyhow::{Context, Result};
use chrono_humanize::Humanize;
use rand::prelude::IteratorRandom;
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client,
    http::Http,
    model::{
        channel::Message,
        guild::{Emoji, Guild},
        id::{ChannelId, GuildId},
        prelude::User,
    },
};

pub trait UserExt {
    fn name_with_disc(&self) -> String;
    fn name_with_disc_and_id(&self) -> String;
    fn avatar_or_default(&self) -> String;
}

impl UserExt for User {
    fn name_with_disc(&self) -> String {
        format!("{}#{}", self.name, self.discriminator)
    }
    fn name_with_disc_and_id(&self) -> String {
        format!("{}#{}({})", self.name, self.discriminator, self.id)
    }

    fn avatar_or_default(&self) -> String {
        self.avatar_url().unwrap_or(self.default_avatar_url())
    }
}

#[async_trait]
pub trait GuildExt {
    async fn stare_emoji(&self, ctx: &client::Context) -> Vec<Emoji>;
}

#[async_trait]
impl GuildExt for Guild {
    async fn stare_emoji(&self, ctx: &client::Context) -> Vec<Emoji> {
        self.id.stare_emoji(&ctx).await
    }
}
#[async_trait]
impl GuildExt for GuildId {
    async fn stare_emoji(&self, ctx: &client::Context) -> Vec<Emoji> {
        self.emojis(&ctx)
            .await
            .map(|emoji| {
                emoji
                    .into_iter()
                    .filter(|e| e.name.starts_with("stare"))
                    .collect()
            })
            .unwrap_or_else(|_| Vec::new())
    }
}

#[async_trait]
pub trait MessageExt {
    async fn reply_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync;
}

#[async_trait]
impl MessageExt for Message {
    async fn reply_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let guild = self.guild(&ctx).await;
        let emoji = if let Some(guild) = guild {
            guild.stare_emoji(&ctx).await
        } else {
            Vec::new()
        };

        self.channel_id
            .send_message(&ctx, move |m| {
                m.reference_message(self);
                m.embed(move |e| {
                    build(e);
                    e.footer(|f| {
                        let mut rng = rand::thread_rng();
                        let emoji = emoji.into_iter().choose(&mut rng);
                        if let Some(emoji) = emoji {
                            f.icon_url(emoji.url());
                        }
                        f.text("TODO put time here LMAO")
                    })
                })
            })
            .await
            .context("Failed to send embed")
    }
}
