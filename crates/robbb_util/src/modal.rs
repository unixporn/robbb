use poise::serenity_prelude::UserId;
use serenity::{
    all::{CommandInteraction, ComponentInteraction},
    builder::CreateInteractionResponse,
    collector::ModalInteractionCollector,
};

use crate::prelude::{AppCtx, Res};

async fn wait_for_modal_ir_response<T: poise::Modal>(ctx: AppCtx<'_>, user_id: UserId) -> Res<T> {
    ctx.has_sent_initial_response.store(true, std::sync::atomic::Ordering::SeqCst);
    let Some(response) =
        ModalInteractionCollector::new(ctx.serenity_context()).author_id(user_id).await
    else {
        anyhow::bail!("No modal interaction response received");
    };

    // Send acknowledgement so that the pop-up is closed
    response
        .create_response(
            ctx.serenity_context(),
            //CreateInteractionResponse::Defer(CreateInteractionResponseMessage::default()),
            // TODO: apparently this is not officially supported for modals, but it works either way.
            CreateInteractionResponse::Acknowledge,
        )
        .await?;

    Ok(T::parse(response.data.clone()).map_err(serenity::Error::Other)?)
}

pub async fn create_modal_component_ir<T: poise::Modal>(
    ctx: AppCtx<'_>,
    interaction: &ComponentInteraction,
    defaults: Option<T>,
) -> Res<T> {
    interaction
        .create_response(&ctx.serenity_context(), T::create(defaults, String::new()))
        .await?;
    wait_for_modal_ir_response(ctx, interaction.user.id).await
}

pub async fn create_modal_command_ir<T: poise::Modal>(
    ctx: AppCtx<'_>,
    interaction: &CommandInteraction,
    defaults: Option<T>,
) -> Res<T> {
    interaction
        .create_response(&ctx.serenity_context(), T::create(defaults, String::new()))
        .await?;
    wait_for_modal_ir_response(ctx, interaction.user.id).await
}
