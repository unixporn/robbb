use crate::{
    config::Config,
    embeds::{self, make_create_embed},
    prelude::{BoxedCreateEmbedBuilder, BoxedCreateMessageBuilder, Ctx},
    UpEmotes,
};

use anyhow::{Context, Result};
use itertools::Itertools;
use poise::ReplyHandle;
use robbb_db::Db;
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
    prelude::Mentionable,
    utils::Colour,
};
use std::{collections::HashMap, fmt::Display, sync::Arc};

type StdResult<T, E> = std::result::Result<T, E>;

#[extend::ext(name = PoiseContextExt)]
#[async_trait]
pub impl<'a> Ctx<'a> {
    fn get_config(&self) -> Arc<Config> {
        self.data().config.clone()
    }

    fn get_db(&self) -> Arc<Db> {
        self.data().db.clone()
    }

    async fn send_embed<F>(&self, build: F) -> StdResult<ReplyHandle<'_>, serenity::Error>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let build_fn: BoxedCreateEmbedBuilder = Box::new(move |e| {
            build(e);
        });
        self.send_embed_full(false, build_fn).await
    }

    async fn send_embed_full<F>(
        &self,
        ephemeral: bool,
        build: F,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let embed = embeds::make_create_embed(&self.discord(), |e| {
            build(e);
            e
        })
        .await;
        self.send(|f| {
            match self {
                poise::Context::Application(_) => {}
                poise::Context::Prefix(prefix) => {
                    f.reference_message(prefix.msg);
                }
            }
            f.embed(|e| {
                *e = embed;
                e
            })
            .ephemeral(ephemeral)
        })
        .await
    }

    async fn say_success(
        &self,
        text: impl Display + Send + Sync + 'static,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        let create_embed = embeds::make_success_embed(&self.discord(), &format!("{}", text)).await;
        self.send_embed(|e| {
            e.clone_from(&create_embed);
        })
        .await
    }

    async fn say_error(
        &self,
        text: impl Display + Send + Sync + 'static,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        let create_embed = embeds::make_error_embed(&self.discord(), &format!("{}", text)).await;
        self.send_embed(|e| {
            e.clone_from(&create_embed);
        })
        .await
    }
    async fn say_success_mod_action(
        &self,
        text: impl Display + Send + Sync + 'static,
    ) -> StdResult<ReplyHandle<'_>, serenity::Error> {
        let create_embed =
            embeds::make_success_mod_action_embed(&self.discord(), &format!("{}", text)).await;
        self.send_embed(|e| {
            e.clone_from(&create_embed);
        })
        .await
    }

    async fn guild_channel(&self) -> anyhow::Result<GuildChannel> {
        Ok(self
            .channel_id()
            .to_channel(&self.discord())
            .await
            .context("Failed to load channel")?
            .guild()
            .context("Failed to load GuildChannel")?)
    }

    fn get_guild_emojis(&self) -> Option<HashMap<EmojiId, Emoji>> {
        Some(self.guild()?.emojis)
    }

    fn get_random_stare(&self) -> Option<Emoji> {
        self.data()
            .up_emotes
            .read()
            .clone()
            .and_then(|x| x.random_stare())
    }
}

#[extend::ext(name = ClientContextExt)]
#[async_trait]
pub impl client::Context {
    async fn get_guild_emojis(&self, id: GuildId) -> Option<HashMap<EmojiId, Emoji>> {
        Some(self.cache.guild(id)?.emojis)
    }

    async fn get_up_emotes(&self) -> Option<Arc<UpEmotes>> {
        self.data.read().await.get::<UpEmotes>().cloned()
    }
    async fn get_config_and_db(&self) -> (Arc<Config>, Arc<Db>) {
        tokio::join!(self.get_config(), self.get_db())
    }

    async fn get_config(&self) -> Arc<Config> {
        self.data.read().await.get::<Config>().cloned().unwrap()
    }
    async fn get_db(&self) -> Arc<Db> {
        self.data.read().await.get::<Db>().cloned().unwrap()
    }
}

#[extend::ext]
pub impl User {
    /// Format a user as `name#discriminator (user-id)`
    fn name_with_disc_and_id(&self) -> String {
        format!("{} ({})", self.tag(), self.id)
    }
    /// Format a user as `@mention (name#discriminator)`
    /// Primarily needed because discord on mobile is bad and doesn't show mentions of users if they're not cached.
    fn mention_and_tag(&self) -> String {
        format!("{} ({})", self.mention(), self.tag())
    }
}

#[extend::ext]
#[async_trait]
pub impl GuildId {
    async fn send_embed<F>(
        &self,
        ctx: &client::Context,
        channel_id: ChannelId,
        build: F,
    ) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let create_embed = make_create_embed(ctx, |e| {
            build(e);
            e
        })
        .await;
        let msg_fn: BoxedCreateMessageBuilder = Box::new(|m| m.set_embed(create_embed));
        Ok(channel_id
            .send_message(&ctx, msg_fn)
            .await
            .context("Failed to send embed message")?)
    }
}

#[extend::ext]
#[async_trait]
pub impl Message {
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
        F: FnOnce(&mut CreateEmbed) -> &mut CreateEmbed + Send + Sync,
    {
        let create_embed = make_create_embed(ctx, |e| build(e)).await;
        let msg = self.clone();
        let msg_fn: BoxedCreateMessageBuilder = Box::new(move |m| {
            m.allowed_mentions(|f| f.replied_user(false));
            m.reference_message(&msg);
            m.set_embed(create_embed)
        });

        self.channel_id
            .send_message(&ctx, msg_fn)
            .await
            .context("Failed to send embed")
    }

    async fn reply_error(
        &self,
        ctx: &client::Context,
        text: impl Display + Send + Sync + 'static,
    ) -> Result<Message> {
        let create_embed = embeds::make_error_embed(ctx, &format!("{}", text)).await;
        self.reply_embed(ctx, |e| {
            e.clone_from(&create_embed);
            e
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

#[extend::ext]
#[async_trait]
pub impl ChannelId {
    async fn send_embed<F>(&self, ctx: &client::Context, build: F) -> Result<Message>
    where
        F: FnOnce(&mut CreateEmbed) + Send + Sync,
    {
        let create_embed = make_create_embed(&ctx, |e| {
            build(e);
            e
        })
        .await;

        let msg_fn: BoxedCreateMessageBuilder = Box::new(|m| m.set_embed(create_embed));
        Ok(self
            .send_message(&ctx, msg_fn)
            .await
            .context("Failed to send embed message")?)
    }
}

#[extend::ext]
pub impl CreateEmbed {
    fn color_opt(&mut self, c: Option<impl Into<Colour>>) -> &mut CreateEmbed {
        if let Some(c) = c {
            self.color(c);
        }
        self
    }

    fn author_user(&mut self, u: User) -> &mut Self {
        self.author(|a| {
            a.name(u.tag())
                .icon_url(u.face())
                .url(format!("https://discord.com/users/{}", u.id))
        })
    }
}

#[extend::ext(name = StrExt)]
pub impl<T: AsRef<str>> T {
    fn split_once_at(&self, c: char) -> Option<(&str, &str)> {
        let s: &str = self.as_ref();
        let index = s.find(c)?;
        Some((&s[..index], &s[index + c.len_utf8()..]))
    }

    /// Splits the string into two parts, separated by the given word.
    /// Ex. `"foo bar baz".split_at_word("bar") // ---> ("foo", "baz")`
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
