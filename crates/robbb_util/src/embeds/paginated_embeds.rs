use crate::{extensions::PoiseContextExt, prelude::Ctx, util::ellipsis_text};

use anyhow::Result;
use itertools::Itertools;
use poise::{serenity_prelude::CreateActionRow, CreateReply, ReplyHandle};
use serenity::{
    builder::{
        CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    },
    collector::ComponentInteractionCollector,
};

const PAGINATION_LEFT: &str = "LEFT";
const PAGINATION_RIGHT: &str = "RIGHT";
const MAX_EMBED_FIELDS: usize = 12; // discords max is 25, but that's ugly

#[derive(Debug)]
pub struct PaginatedEmbed {
    pages: Vec<CreateEmbed<'static>>,
    base_embed: CreateEmbed<'static>,
}

impl PaginatedEmbed {
    pub async fn create(
        embeds: impl IntoIterator<Item = CreateEmbed<'static>>,
        base_embed: CreateEmbed<'static>,
    ) -> PaginatedEmbed {
        PaginatedEmbed { pages: embeds.into_iter().collect(), base_embed }
    }

    pub async fn create_from_fields(
        title: String,
        fields: impl IntoIterator<Item = (String, String)>,
        base_embed: CreateEmbed<'static>,
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
                    e = e.title(title.clone());
                } else {
                    e = e.title(format!("{} ({}/{})", title, page_idx + 1, page_cnt));
                }
                e.fields(fields.map(|(k, v)| (k, ellipsis_text(&v, 500), false)).collect_vec())
            })
            .collect_vec();

        PaginatedEmbed { pages, base_embed }
    }

    pub async fn reply_to(&self, ctx: Ctx<'_>, ephemeral: bool) -> Result<()> {
        match self.pages.as_slice() {
            [] => {
                if ephemeral {
                    ctx.reply_embed_ephemeral(self.base_embed.clone()).await?;
                } else {
                    ctx.reply_embed(self.base_embed.clone()).await?;
                }
            }
            [page] => {
                if ephemeral {
                    ctx.reply_embed_ephemeral(page.clone()).await?;
                } else {
                    ctx.reply_embed(page.clone()).await?;
                }
            }
            pages => {
                let reply = CreateReply::default()
                    .ephemeral(ephemeral)
                    .components(vec![make_paginate_row(ctx.id(), 0, pages.len())])
                    .embed(self.pages.first().unwrap().clone());
                let handle = ctx.send(reply).await?;
                handle_pagination_interactions(ctx, pages.to_vec(), &handle).await?;
            }
        }
        Ok(())
    }
}

#[tracing::instrument(skip_all)]
async fn handle_pagination_interactions(
    ctx: Ctx<'_>,
    pages: Vec<CreateEmbed<'static>>,
    handle: &ReplyHandle<'_>,
) -> Result<()> {
    let mut current_page_idx = 0;
    let ctx_id = ctx.id();

    while let Some(interaction) =
        ComponentInteractionCollector::new(ctx.serenity_context().shard.clone())
            .filter(move |x| x.data.custom_id.starts_with(&ctx_id.to_string()))
            .timeout(std::time::Duration::from_secs(30))
            .author_id(ctx.author().id)
            .await
    {
        let direction = interaction.data.clone().custom_id;
        let left_id = format!("{ctx_id}{PAGINATION_LEFT}");
        let right_id = format!("{ctx_id}{PAGINATION_RIGHT}");
        if direction == left_id && current_page_idx > 0 {
            current_page_idx -= 1;
        } else if direction == right_id && current_page_idx < pages.len() - 1 {
            current_page_idx += 1;
        }
        let response_msg = CreateInteractionResponseMessage::default()
            .embed(pages.get(current_page_idx).unwrap().clone())
            .components(vec![make_paginate_row(ctx_id, current_page_idx, pages.len())]);
        interaction
            .create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(response_msg))
            .await?;
    }
    // Once no further interactions are expected, remove the components from the message
    let reply = CreateReply::default()
        .embed(pages.get(current_page_idx).unwrap().clone())
        .components(vec![]);
    handle.edit(ctx, reply).await?;
    Ok(())
}

fn make_paginate_row<'a>(ctx_id: u64, page_idx: usize, page_cnt: usize) -> CreateActionRow<'a> {
    CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{ctx_id}{PAGINATION_LEFT}")).label("←").disabled(page_idx == 0),
        CreateButton::new(format!("{ctx_id}{PAGINATION_RIGHT}"))
            .label("→")
            .disabled(page_idx >= page_cnt - 1),
    ])
}
