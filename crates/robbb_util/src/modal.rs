use poise::serenity_prelude::{
    ApplicationCommandInteraction, CollectModalInteraction, InteractionResponseType,
    MessageComponentInteraction, UserId,
};

use crate::prelude::{AppCtx, Res};

async fn wait_for_modal_ir_response<T: poise::Modal>(ctx: AppCtx<'_>, user_id: UserId) -> Res<T> {
    ctx.has_sent_initial_response.store(true, std::sync::atomic::Ordering::SeqCst);
    // Wait for user to submit
    let response =
        CollectModalInteraction::new(&ctx.discord.shard).author_id(user_id).await.unwrap();

    // Send acknowledgement so that the pop-up is closed
    response
        .create_interaction_response(ctx.discord, |b| {
            b.kind(InteractionResponseType::DeferredUpdateMessage)
        })
        .await?;

    Ok(T::parse(response.data.clone()).map_err(serenity::Error::Other)?)
}

pub async fn create_modal_component_ir<T: poise::Modal>(
    ctx: AppCtx<'_>,
    interaction: &MessageComponentInteraction,
    defaults: Option<T>,
) -> Res<T> {
    interaction
        .create_interaction_response(&ctx.discord, |ir| {
            *ir = T::create(defaults);
            ir
        })
        .await?;
    wait_for_modal_ir_response(ctx, interaction.user.id).await
}

pub async fn create_modal_command_ir<T: poise::Modal>(
    ctx: AppCtx<'_>,
    interaction: &ApplicationCommandInteraction,
    defaults: Option<T>,
) -> Res<T> {
    interaction
        .create_interaction_response(&ctx.discord, |ir| {
            *ir = T::create(defaults);
            ir
        })
        .await?;
    wait_for_modal_ir_response(ctx, interaction.user.id).await
}