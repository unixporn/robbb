use std::sync::Arc;

use futures::StreamExt;
use poise::serenity_prelude::{
    interaction::{message_component::MessageComponentInteraction, InteractionResponseType},
    ComponentInteractionCollector, Message, UserId,
};
use serenity::client;

pub struct UserSpecificComponentInteractionCollector {
    inner_collector: ComponentInteractionCollector,
    user_id: UserId,
    by_user_limit: usize,
}

impl UserSpecificComponentInteractionCollector {
    pub async fn next(
        &mut self,
        ctx: &client::Context,
    ) -> Option<Arc<MessageComponentInteraction>> {
        loop {
            if self.by_user_limit == 0 {
                return None;
            }
            match self.inner_collector.next().await {
                Some(interaction) => {
                    if interaction.user.id.0 == self.user_id.0 {
                        self.by_user_limit -= 1;
                        return Some(interaction);
                    } else {
                        let _ = interaction
                            .create_interaction_response(ctx, |ir| {
                                ir.kind(InteractionResponseType::ChannelMessageWithSource);
                                ir.interaction_response_data(|ir| {
                                    ir.content("This is not your button!").ephemeral(true)
                                })
                            })
                            .await;
                    }
                }
                None => return None,
            }
        }
    }
}

pub fn await_component_interactions_by(
    ctx: &client::Context,
    message: &Message,
    user_id: UserId,
    by_user_limit: usize,
    timeout: std::time::Duration,
) -> UserSpecificComponentInteractionCollector {
    let collector = message.await_component_interactions(&ctx).timeout(timeout).build();
    UserSpecificComponentInteractionCollector { user_id, by_user_limit, inner_collector: collector }
}
