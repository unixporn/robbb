use crate::{extensions::PoiseContextExt, prelude::Ctx, UpEmotes};

use anyhow::Result;
use chrono::Utc;
use itertools::Itertools;
use serenity::{
    builder::CreateEmbed,
    client,
    futures::StreamExt,
    model::channel::{Message, ReactionType},
};
use tracing_futures::Instrument;

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
                .send_embed(|e| e.clone_from(self.pages.get(current_page_idx).unwrap()))
                .await?;
            let created_msg = created_msg_handle.message().await?;

            let serenity_ctx = ctx.discord().clone();
            let user_id = ctx.author().id;

            tokio::spawn({
                let mut created_msg = created_msg.clone();
                let created_msg_id = created_msg.id;
                async move {
                    let res: Result<()> = async move {
                        let emoji_left = ReactionType::from('◀');
                        let emoji_right = ReactionType::from('▶');

                        let reaction_left =
                            created_msg.react(&serenity_ctx, emoji_left.clone()).await?;
                        let reaction_right = created_msg
                            .react(&serenity_ctx, emoji_right.clone())
                            .await?;

                        let mut collector = created_msg
                            .await_reactions(&serenity_ctx)
                            .timeout(std::time::Duration::from_secs(30))
                            .collect_limit(10)
                            .author_id(user_id)
                            .filter(move |r| {
                                r.emoji == reaction_left.emoji || r.emoji == reaction_right.emoji
                            })
                            .build();

                        while let Some(reaction) = collector.next().await {
                            let reaction = &reaction.as_ref().as_inner_ref();
                            let emoji = &reaction.emoji;

                            if emoji == &emoji_left && current_page_idx > 0 {
                                current_page_idx -= 1;
                            } else if emoji == &emoji_right && current_page_idx < pages.len() - 1 {
                                current_page_idx += 1;
                            }
                            created_msg
                                .edit(&serenity_ctx, |e| {
                                    e.set_embed(pages.get(current_page_idx).unwrap().clone())
                                })
                                .await?;
                            reaction.delete(&serenity_ctx).await?;
                        }

                        created_msg.delete_reactions(&serenity_ctx).await?;
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

// TODORW (how do I set the interaction as handled without needing to send a message?)
/*

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

                            created_msg
                                .edit(&serenity_ctx, |e| {
                                    e.set_embed(pages.get(current_page_idx).unwrap().clone())
                                })
                                .await?;
                        }

                        //created_msg.delete_reactions(&serenity_ctx).await?;
*/
