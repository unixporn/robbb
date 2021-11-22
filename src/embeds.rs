use crate::extensions::*;

use anyhow::*;
use chrono::Utc;
use itertools::Itertools;
use serenity::{
    builder::{CreateEmbed, CreateMessage},
    client,
    futures::StreamExt,
    model::channel::{Message, ReactionType},
};
use tracing_futures::Instrument;

#[derive(Debug)]
pub struct PaginatedEmbed {
    embeds: Vec<CreateEmbed>,
    base_embed: CreateEmbed,
}

impl PaginatedEmbed {
    pub async fn create(
        embeds: impl IntoIterator<Item = CreateEmbed>,
        base_embed: CreateEmbed,
    ) -> PaginatedEmbed {
        PaginatedEmbed {
            embeds: embeds.into_iter().collect(),
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

        PaginatedEmbed {
            embeds: pages,
            base_embed,
        }
    }

    #[tracing::instrument(skip_all, fields(?self, %msg.id))]
    pub async fn reply_to(&self, ctx: &client::Context, msg: &Message) -> Result<Message> {
        let pages = self.embeds.iter();
        let pages = pages
            .map(|e| {
                let mut m = CreateMessage::default();
                m.set_embed(e.clone());
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
                        m.set_embed(self.base_embed.clone());
                    }
                    m.reference_message(msg)
                })
                .await?)
        } else {
            let mut current_page_idx = 0;
            let created_msg = msg
                .channel_id
                .send_message(&ctx, |m| {
                    m.clone_from(pages.get(current_page_idx).unwrap());
                    m.reference_message(msg)
                })
                .await?;

            let ctx = ctx.clone();
            let user_id = msg.author.id;

            tokio::spawn({
                let mut created_msg = created_msg.clone();
                let created_msg_id = created_msg.id;
                async move {
                    let res: Result<()> = async move {
                        let emoji_left = ReactionType::from('◀');
                        let emoji_right = ReactionType::from('▶');

                        let reaction_left = created_msg.react(&ctx, emoji_left.clone()).await?;
                        let reaction_right = created_msg.react(&ctx, emoji_right.clone()).await?;

                        let mut collector = created_msg
                            .await_reactions(&ctx)
                            .timeout(std::time::Duration::from_secs(30))
                            .collect_limit(10)
                            .author_id(user_id)
                            .filter(move |r| {
                                r.emoji == reaction_left.emoji || r.emoji == reaction_right.emoji
                            })
                            .await;

                        while let Some(reaction) = collector.next().await {
                            let reaction = &reaction.as_ref().as_inner_ref();
                            let emoji = &reaction.emoji;

                            if emoji == &emoji_left && current_page_idx > 0 {
                                current_page_idx -= 1;
                            } else if emoji == &emoji_right && current_page_idx < pages.len() - 1 {
                                current_page_idx += 1;
                            }
                            created_msg
                                .edit(&ctx, |e| {
                                    e.0.clone_from(&pages.get(current_page_idx).unwrap().0);
                                    e
                                })
                                .await?;
                            reaction.delete(&ctx).await?;
                        }

                        created_msg.delete_reactions(&ctx).await?;
                        Ok(())
                    }
                    .instrument(
                        tracing::info_span!("paginate-embed", embed_msg.id = %created_msg_id),
                    )
                    .await;
                    if let Err(err) = res {
                        log::error!("{}", err);
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
    let stare = ctx.get_random_stare().await;

    let mut e = CreateEmbed::default();

    e.timestamp(&Utc::now());
    e.footer(|f| {
        if let Some(emoji) = stare {
            f.icon_url(emoji.url());
        }
        f.text("\u{200b}")
    });

    build(&mut e);
    e
}
