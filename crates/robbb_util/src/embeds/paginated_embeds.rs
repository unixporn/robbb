use crate::{extensions::PoiseContextExt, log_error, prelude::Ctx, util::ellipsis_text};

use anyhow::Result;
use itertools::Itertools;
use poise::serenity_prelude::{CreateActionRow, CreateComponents, UserId};
use serenity::{builder::CreateEmbed, client, model::channel::Message};

const PAGINATION_LEFT: &str = "LEFT";
const PAGINATION_RIGHT: &str = "RIGHT";
const MAX_EMBED_FIELDS: usize = 12; // discords max is 25, but that's ugly

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
        PaginatedEmbed { pages: embeds.into_iter().collect(), base_embed }
    }

    pub async fn create_from_fields(
        title: String,
        fields: impl IntoIterator<Item = (String, String)>,
        base_embed: CreateEmbed,
    ) -> PaginatedEmbed {
        let pages = fields.into_iter().chunks(MAX_EMBED_FIELDS);
        let pages: Vec<_> = pages.into_iter().collect();
        let page_cnt = pages.len();
        let pages = pages
            .into_iter()
            .enumerate()
            .map(|(page_idx, fields)| {
                let mut e = base_embed.clone();
                if page_cnt < 2 {
                    e.title(&title);
                } else {
                    e.title(format!("{} ({}/{})", title, page_idx + 1, page_cnt));
                }
                e.fields(fields.map(|(k, v)| (k, ellipsis_text(&v, 500), false)).collect_vec());
                e
            })
            .collect_vec();

        PaginatedEmbed { pages, base_embed }
    }

    #[tracing::instrument(name = "send_paginated_embed", skip_all, fields(paginated_embed.page_cnt = %self.pages.len()))]
    pub async fn reply_to(&self, ctx: Ctx<'_>, ephemeral: bool) -> Result<Message> {
        let pages = self.pages.clone();
        match pages.len() {
            0 => {
                let handle =
                    ctx.send_embed_full(ephemeral, |e| e.clone_from(&self.base_embed)).await?;
                Ok(handle.message().await?)
            }
            1 => {
                let page = self.pages.first().unwrap();
                let handle = ctx.send_embed_full(ephemeral, |e| e.clone_from(page)).await?;
                Ok(handle.message().await?)
            }
            _ => {
                let created_msg_handle = ctx
                    .send(|m| {
                        m.embeds.push(self.pages.get(0).unwrap().clone());
                        m.components = Some(make_paginate_components(0, pages.len()));
                        m.ephemeral(ephemeral);
                        m
                    })
                    .await?;
                let created_msg = created_msg_handle.message().await?;

                tokio::spawn({
                    let serenity_ctx = ctx.discord().clone();
                    let user_id = ctx.author().id;
                    let created_msg = created_msg.clone();
                    async move {
                        log_error!(
                            handle_pagination_interactions(
                                &serenity_ctx,
                                pages,
                                user_id,
                                created_msg
                            )
                            .await
                        )
                    }
                });

                Ok(created_msg)
            }
        }
    }
}

#[tracing::instrument(skip_all)]
async fn handle_pagination_interactions(
    serenity_ctx: &client::Context,
    pages: Vec<CreateEmbed>,
    user_id: UserId,
    mut created_msg: Message,
) -> Result<()> {
    let mut current_page_idx = 0;

    let mut interactions = crate::collect_interaction::await_component_interactions_by(
        serenity_ctx,
        &created_msg,
        user_id,
        10,
        std::time::Duration::from_secs(30),
    );

    while let Some(interaction) = interactions.next(serenity_ctx).await {
        let direction = interaction.data.clone().custom_id;
        if direction == PAGINATION_LEFT && current_page_idx > 0 {
            current_page_idx -= 1;
        } else if direction == PAGINATION_RIGHT && current_page_idx < pages.len() - 1 {
            current_page_idx += 1;
        }
        interaction
            .create_interaction_response(&serenity_ctx, |ir| {
                ir.kind(poise::serenity_prelude::InteractionResponseType::UpdateMessage);
                ir.interaction_response_data(|d| {
                    d.set_embed(pages.get(current_page_idx).unwrap().clone());
                    d.set_components(make_paginate_components(current_page_idx, pages.len()))
                })
            })
            .await?;
    }
    created_msg
        .edit(&serenity_ctx, |e| {
            e.set_embed(pages.get(current_page_idx).unwrap().clone());
            e.components(|c| c)
        })
        .await?;
    Ok(())
}

fn make_paginate_components(page_idx: usize, page_cnt: usize) -> CreateComponents {
    let mut row = CreateActionRow::default();
    row.create_button(|b| b.label("←").disabled(page_idx == 0).custom_id(PAGINATION_LEFT));
    row.create_button(|b| {
        b.label("→").disabled(page_idx >= page_cnt - 1).custom_id(PAGINATION_RIGHT)
    });
    let mut components = CreateComponents::default();
    components.set_action_row(row);
    components
}
