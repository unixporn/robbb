use crate::{
    extensions::{ClientContextExt, PoiseContextExt},
    prelude::Ctx,
    UpEmotes,
};

use anyhow::Result;
use chrono::Utc;
use itertools::Itertools;
use serenity::{builder::CreateEmbed, client, futures::StreamExt, model::channel::Message};
use tracing_futures::Instrument;

const PAGINATION_LEFT: &str = "LEFT";
const PAGINATION_RIGHT: &str = "RIGHT";

#[derive(Debug)]
pub struct PaginatedEmbed {
    pages: Vec<CreateEmbed>,
    base_embed: CreateEmbed,
}

impl PaginatedEmbed {
    pub async fn create(
        embeds: impl IntoIterator<Item = CreateEmbed>,
        base_embed: CreateEmbed,
    ) -> PaginatedEmbed {
        PaginatedEmbed {
            pages: embeds.into_iter().collect(),
            base_embed,
        }
    }

    pub async fn create_from_fields(
        fields: impl IntoIterator<Item = (String, String)>,
        base_embed: CreateEmbed,
    ) -> PaginatedEmbed {
        let pages = fields.into_iter().chunks(25);
        let pages = pages
            .into_iter()
            .map(|fields| {
                let mut e = base_embed.clone();
                e.fields(fields.map(|(k, v)| (k, v, false)).collect_vec());
                e
            })
            .collect_vec();

        PaginatedEmbed { pages, base_embed }
    }

    #[tracing::instrument(skip_all, fields(?self))]
    pub async fn reply_to(&self, ctx: Ctx<'_>) -> Result<Message> {
        let pages = self.pages.clone();
        if pages.len() == 0 {
            let handle = ctx.send_embed(|e| e.clone_from(&self.base_embed)).await?;
            Ok(handle.message().await?)
        } else if pages.len() == 1 {
            let handle = ctx
                .send_embed(|e| e.clone_from(self.pages.first().unwrap()))
                .await?;
            Ok(handle.message().await?)
        } else {
            let mut current_page_idx = 0;
            let created_msg_handle = ctx
                .send(|e| {
                    e.embed(|e| {
                        e.clone_from(&self.pages.get(current_page_idx).unwrap());
                        e
                    });
                    e.components(|c| {
                        c.create_action_row(|r| {
                            r.create_button(|b| b.label("←").custom_id(PAGINATION_LEFT));
                            r.create_button(|b| b.label("→").custom_id(PAGINATION_RIGHT))
                        })
                    })
                })
                .await?;
            let created_msg = created_msg_handle.message().await?;

            let serenity_ctx = ctx.discord().clone();
            let user_id = ctx.author().id;

            tokio::spawn({
                let mut created_msg = created_msg.clone();
                let created_msg_id = created_msg.id;
                async move {
                    let res: Result<()> =
                        async move {
                            let mut interaction_stream = created_msg
                                .await_component_interactions(&serenity_ctx)
                                .collect_limit(10)
                                .timeout(std::time::Duration::from_secs(30))
                                .author_id(user_id)
                                .build();

                            while let Some(interaction) = interaction_stream.next().await {
                                let direction = interaction.data.clone().custom_id;
                                if direction == PAGINATION_LEFT && current_page_idx > 0 {
                                    current_page_idx -= 1;
                                } else if direction == PAGINATION_RIGHT
                                    && current_page_idx < pages.len() - 1
                                {
                                    current_page_idx += 1;
                                }
                                interaction.create_interaction_response(&serenity_ctx, |ir| {
                                    ir.kind(poise::serenity_prelude::InteractionResponseType::UpdateMessage);
                                    ir.interaction_response_data(|d| {
                                        d.set_embed(pages.get(current_page_idx).unwrap().clone())
                                    })
                                }).await?;

                            }
                            created_msg
                                .edit(&serenity_ctx, |e| e.components(|c| c))
                                .await?;
                            Ok(())
                        }
                        .instrument(
                            tracing::info_span!("paginate-embed", embed_msg.id = %created_msg_id),
                        )
                        .await;
                    if let Err(err) = res {
                        tracing::error!("{}", err);
                    }
                }
                .instrument(
                    tracing::info_span!("paginate-embed-outer", embed_msg.id = %created_msg_id),
                )
            });

            Ok(created_msg)
        }
    }
}

pub async fn make_create_embed(
    ctx: &client::Context,
    build: impl FnOnce(&mut CreateEmbed) -> &mut CreateEmbed,
) -> CreateEmbed {
    let stare = ctx
        .data
        .read()
        .await
        .get::<UpEmotes>()
        .and_then(|x| x.random_stare());

    let mut e = CreateEmbed::default();

    e.timestamp(Utc::now());
    e.footer(|f| {
        if let Some(emoji) = stare {
            f.icon_url(emoji.url());
        }
        f.text("\u{200b}")
    });

    build(&mut e);
    e
}

pub async fn make_success_embed(
    ctx: &client::Context,
    text: impl std::fmt::Display + Send + Sync + 'static,
) -> CreateEmbed {
    let emote = ctx
        .get_up_emotes()
        .await
        .as_ref()
        .map(|x| format!(" {}", x.poggers.clone()));

    let mut e = CreateEmbed::default();
    e.description(format!("{}{}", text, emote.unwrap_or_default()));
    e.color(0xb8bb26u32);
    e
}

pub async fn make_success_mod_action_embed(
    ctx: &client::Context,
    text: impl std::fmt::Display + Send + Sync + 'static,
) -> CreateEmbed {
    let emote = ctx
        .get_up_emotes()
        .await
        .as_ref()
        .map(|x| format!(" {}", x.police.clone()));

    let mut e = CreateEmbed::default();
    e.description(format!("{}{}", text, emote.unwrap_or_default()));
    e.color(0xb8bb26u32);
    e
}

pub async fn make_error_embed(
    ctx: &client::Context,
    text: impl std::fmt::Display + Send + Sync + 'static,
) -> CreateEmbed {
    let emote = ctx
        .get_up_emotes()
        .await
        .as_ref()
        .map(|x| format!(" {}", x.pensibe.clone()));

    let mut e = CreateEmbed::default();
    e.description(format!("{}{}", text, emote.unwrap_or_default()));
    e.color(0xfb4934u32);
    e
}
