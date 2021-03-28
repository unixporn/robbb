use crate::extensions::*;
use std::sync::Arc;

use anyhow::*;
use chrono::Utc;
use itertools::Itertools;
use serenity::{
    builder::{CreateEmbed, CreateMessage},
    client,
    model::channel::{Message, ReactionType},
};
use serenity_utils::menu::*;

pub struct PaginatedFieldsEmbed {
    create_embed: CreateEmbed,
    fields: Vec<(String, String)>,
}

impl PaginatedFieldsEmbed {
    pub async fn create(
        ctx: &client::Context,
        fields: impl IntoIterator<Item = (String, String)>,
        build: impl FnOnce(&mut CreateEmbed),
    ) -> PaginatedFieldsEmbed {
        let mut embed = basic_create_embed(&ctx).await;
        build(&mut embed);
        PaginatedFieldsEmbed {
            create_embed: embed,
            fields: fields.into_iter().collect(),
        }
    }

    pub async fn send(&self, ctx: &client::Context, msg: &Message) -> Result<Message> {
        let pages = self.fields.iter().chunks(25);
        let pages = pages
            .into_iter()
            .map(|fields| {
                let mut m = CreateMessage::default();
                let mut e = self.create_embed.clone();
                e.fields(fields.map(|(k, v)| (k, v, false)).collect_vec());
                m.set_embed(e);
                m
            })
            .collect_vec();

        if pages.len() < 2 {
            Ok(msg
                .channel_id
                .send_message(&ctx, |m: &mut CreateMessage| {
                    if let Some(create_message) = pages.first() {
                        m.clone_from(create_message);
                    } else {
                        m.set_embed(self.create_embed.clone());
                    }
                    m.reference_message(msg)
                })
                .await?)
        } else {
            let controls = vec![
                Control::new(
                    ReactionType::from('◀'),
                    Arc::new(|m, r| Box::pin(prev_page(m, r))),
                ),
                Control::new(
                    ReactionType::from('▶'),
                    Arc::new(|m, r| Box::pin(next_page(m, r))),
                ),
            ];

            let options = MenuOptions {
                controls,
                ..Default::default()
            };

            let menu = Menu::new(ctx, msg, &pages, options);
            Ok(menu.run().await?.context("No paginated message sent")?)
        }
    }
}

pub async fn basic_create_embed(ctx: &client::Context) -> CreateEmbed {
    let stare = ctx.get_random_stare().await;

    let mut e = CreateEmbed::default();

    e.timestamp(&Utc::now());
    e.footer(|f| {
        if let Some(emoji) = stare {
            f.icon_url(emoji.url());
        }
        f.text("\u{200b}")
    });
    e
}
