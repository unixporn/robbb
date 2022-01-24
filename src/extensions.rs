use crate::{db::Db, embeds::make_create_embed, Config, UpEmotes};
use anyhow::{Context, Result};
use itertools::Itertools;
use serenity::{
    async_trait,
    builder::CreateEmbed,
    client,
    model::{
        channel::{GuildChannel, Message},
        guild::Emoji,
        id::{ChannelId, EmojiId, GuildId},
        prelude::User,
    },
    utils::Colour,
};
use std::{collections::HashMap, fmt::Display, sync::Arc};

#[async_trait]
pub trait ClientContextExt {
    async fn get_config(&self) -> Arc<Config>;

    async fn get_db(&self) -> Arc<Db>;

    async fn get_config_and_db(&self) -> (Arc<Config>, Arc<Db>);

    async fn get_up_emotes(&self) -> Option<Arc<UpEmotes>>;

    async fn get_guild_emojis(&self, id: GuildId) -> Option<HashMap<EmojiId, Emoji>>;

    async fn get_random_stare(&self) -> Option<Emoji>;
}

#[async_trait]
impl ClientContextExt for client::Context {
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

    async fn get_up_emotes(&self) -> Option<Arc<UpEmotes>> {
        self.data.read().await.get::<UpEmotes>().cloned()
    }

    async fn get_random_stare(&self) -> Option<Emoji> {
        self.get_up_emotes().await?.random_stare()
    }

    async fn get_guild_emojis(&self, id: GuildId) -> Option<HashMap<EmojiId, Emoji>> {
        Some(self.cache.guild(id).await?.emojis)
    }
}

#[async_trait]
pub trait UserExt {
    fn name_with_disc_and_id(&self) -> String;
}

impl UserExt for User {
    fn name_with_disc_and_id(&self) -> String {
        format!("{}({})", self.tag(), self.id)
    }
}

#[async_trait]
pub trait GuildIdExt {
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
impl GuildIdExt for GuildId {
    async fn send_embed<F>(
        &self,
        ctx: &client::Context,
        channel_id: ChannelId,
        build: F,
    ) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let create_embed = make_create_embed(&ctx, |e| {
            build(e);
            e
        })
        .await;
        Ok(channel_id
            .send_message(&ctx, |m| m.set_embed(create_embed))
            .await
            .context("Failed to send embed message")?)
    }
}

#[async_trait]
pub trait MessageExt {
    fn find_image_urls(&self) -> Vec<String>;

    async fn reply_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync;

    async fn reply_error(
        &self,
        ctx: &client::Context,
        s: impl Display + Send + Sync + 'static,
    ) -> Result<Message>;

    async fn reply_success(
        &self,
        ctx: &client::Context,
        s: impl Display + Send + Sync + 'static,
    ) -> Result<Message>;

    async fn reply_success_mod_action(
        &self,
        ctx: &client::Context,
        s: impl Display + Send + Sync + 'static,
    ) -> Result<Message>;

    fn to_context_link(&self) -> String;

    async fn create_thread(
        &self,
        ctx: &client::Context,
        title: impl Display + Send + Sync + 'static,
    ) -> Result<GuildChannel>;
}

#[async_trait]
impl MessageExt for Message {
    fn find_image_urls(&self) -> Vec<String> {
        self.embeds
            .iter()
            .filter_map(|embed| embed.image.clone())
            .map(|image| image.url)
            .chain(
                self.attachments
                    .iter()
                    .filter(|a| a.dimensions().is_some() && crate::util::is_image_file(&a.filename))
                    .map(|a| a.url.to_string()),
            )
            .collect_vec()
    }

    async fn reply_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let create_embed = make_create_embed(&ctx, |e| {
            build(e);
            e
        })
        .await;

        self.channel_id
            .send_message(&ctx, move |m| {
                m.allowed_mentions(|f| f.replied_user(false));
                m.reference_message(self);
                m.set_embed(create_embed)
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
            .map(|x| format!(" {}", x.pensibe.clone()));
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
            .map(|x| format!(" {}", x.poggers.clone()));

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
            .map(|x| format!(" {}", x.police.clone()));

        self.reply_embed(&ctx, |e| {
            e.description(format!("{}{}", s, police.unwrap_or_default()));
            e.color(0xb8bb26);
        })
        .await
    }

    async fn create_thread(
        &self,
        ctx: &client::Context,
        title: impl Display + Send + Sync + 'static,
    ) -> Result<GuildChannel> {
        self.channel(&ctx)
            .await
            .context("Failed to fetch message channel")?
            .guild()
            .context("Failed to request guild channel")?
            .create_public_thread(&ctx, self, |e| e.name(title))
            .await
            .context("Failed to create a thread")
    }

    fn to_context_link(&self) -> String {
        format!("[(context)]({})", self.link())
    }
}

#[async_trait]
pub trait ChannelIdExt {
    async fn send_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync;

    async fn send_error(
        &self,
        ctx: &client::Context,
        s: impl Display + Send + Sync + 'static,
    ) -> Result<Message> {
        let pensibe = ctx
            .get_up_emotes()
            .await
            .map(|x| format!(" {}", x.pensibe.clone()));
        self.send_embed(&ctx, |e| {
            e.description(format!("{}{}", s, pensibe.unwrap_or_default()));
            e.color(0xfb4934);
        })
        .await
    }
}

#[async_trait]
impl ChannelIdExt for ChannelId {
    async fn send_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let create_embed = make_create_embed(&ctx, |e| {
            build(e);
            e
        })
        .await;
        Ok(self
            .send_message(&ctx, |m| m.set_embed(create_embed))
            .await
            .context("Failed to send embed message")?)
    }
}

#[async_trait]
pub trait CreateEmbedExt {
    fn color_opt(&mut self, c: Option<impl Into<Colour>>) -> &mut CreateEmbed;
}

impl CreateEmbedExt for CreateEmbed {
    fn color_opt(&mut self, c: Option<impl Into<Colour>>) -> &mut CreateEmbed {
        if let Some(c) = c {
            self.color(c);
        }
        self
    }
}

#[async_trait]
pub trait StrExt<T: AsRef<str>> {
    fn split_once_at(&self, c: char) -> Option<(&str, &str)>;

    /// Splits the string into two parts, separated by the given word.
    /// Ex. `"foo bar baz".split_at_word("bar") // ---> ("foo", "baz")`
    fn split_at_word(&self, split_at: &str) -> (String, String);
}

impl<T: AsRef<str>> StrExt<T> for T {
    fn split_once_at(&self, c: char) -> Option<(&str, &str)> {
        let s: &str = self.as_ref();
        let index = s.find(c)?;
        Some((&s[..index], &s[index + c.len_utf8()..]))
    }

    fn split_at_word(&self, split_at: &str) -> (String, String) {
        let mut words = self.as_ref().trim().split(' ').collect_vec();
        match words.iter().position(|w| w == &split_at) {
            Some(word_ind) => {
                let right_side = words.split_off(word_ind + 1).join(" ");
                words.pop();
                (words.join(" "), right_side)
            }
            None => (String::from(self.as_ref()), String::new()),
        }
    }
}
