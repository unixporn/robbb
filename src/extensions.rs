use anyhow::{Context, Result};
use chrono::Utc;
use chrono_humanize::Humanize;
use rand::prelude::IteratorRandom;
use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateMessage},
    client,
    http::Http,
    model::{
        channel::Message,
        guild::{Emoji, Guild},
        id::{ChannelId, GuildId, MessageId, UserId},
        prelude::User,
    },
};

pub trait UserExt {
    fn name_with_disc_and_id(&self) -> String;
    fn avatar_or_default(&self) -> String;
}

impl UserExt for User {
    fn name_with_disc_and_id(&self) -> String {
        format!("{}#{}({})", self.name, self.discriminator, self.id)
    }

    fn avatar_or_default(&self) -> String {
        self.avatar_url().unwrap_or(self.default_avatar_url())
    }
}

#[async_trait]
pub trait GuildExt {
    async fn random_stare_emoji(&self, ctx: &client::Context) -> Option<Emoji>;
    async fn send_embed<F>(
        &self,
        ctx: &client::Context,
        channel_id: ChannelId,
        build: F,
    ) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync;
}

#[async_trait]
impl GuildExt for Guild {
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
#[async_trait]
impl GuildExt for GuildId {
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
        let build_basics = build_embed_builder(&ctx, *self).await;
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
        let build_basics = if let Some(guild_id) = self.guild_id {
            Some(build_embed_builder(&ctx, guild_id).await)
        } else {
            None
        };

        self.channel_id
            .send_message(&ctx, move |m| {
                m.reference_message(self);
                m.embed(move |e| {
                    if let Some(build_basics) = build_basics {
                        build_basics(e);
                    }
                    build(e);
                    e
                })
            })
            .await
            .context("Failed to send embed")
    }
}

async fn build_embed_builder(
    ctx: &client::Context,
    guild_id: GuildId,
) -> impl FnOnce(&mut CreateEmbed) {
    let guild = guild_id.to_guild_cached(&ctx).await;
    let emoji = if let Some(guild) = guild {
        guild.random_stare_emoji(&ctx).await
    } else {
        None
    };

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
