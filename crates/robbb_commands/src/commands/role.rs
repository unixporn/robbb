use anyhow::Context;
use poise::serenity_prelude::ComponentInteractionDataKind;
use poise::CreateReply;
use robbb_util::embeds;
use serenity::{
    all::RoleId,
    builder::{
        CreateActionRow, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
        CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    },
};

use super::*;

/// Select a role. The role can be selected in a popup.
#[poise::command(slash_command, guild_only)]
pub async fn role(ctx: Ctx<'_>) -> Res<()> {
    const NONE_VALUE: &str = "NONE";
    let config = ctx.get_config();

    let guild = ctx.guild().context("Not in a guild")?.to_owned();
    let available_roles = std::iter::once(("None".to_string(), NONE_VALUE.to_string())).chain(
        config
            .roles_color
            .iter()
            .filter_map(|r| guild.roles.get(r))
            .map(|r| (r.name.to_string(), r.id.get().to_string())),
    );
    let interaction_custom_id = format!("{}-role", ctx.id());

    let handle = ctx
        .send({
            let embed = CreateEmbed::default()
                .title("Available roles")
                .description(config.roles_color.iter().map(|r| r.mention()).join(" "));
            let options =
                available_roles.map(|(name, id)| CreateSelectMenuOption::new(name, id)).collect();
            let menu = CreateSelectMenu::new(
                &interaction_custom_id,
                CreateSelectMenuKind::String { options },
            )
            .min_values(1)
            .max_values(1);
            CreateReply::default()
                .embed(embed)
                .components(vec![CreateActionRow::SelectMenu(menu)])
                .ephemeral(true)
        })
        .await?;
    let mut roles_msg = handle.message().await?;

    if let Some(interaction) = roles_msg
        .to_mut()
        .await_component_interactions(ctx.serenity_context())
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(30))
        .custom_ids(vec![interaction_custom_id])
        .await
    {
        interaction
            .create_response(
                &ctx,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::default()
                        .embed(embeds::base_embed(&ctx).description("Updating roles..."))
                        .components(vec![]),
                ),
            )
            .await
            .context("Failed to create interactionresponse")?;
        let selected: String = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                values.first().context("Nothing selected")?.to_string()
            }
            _ => anyhow::bail!("Wrong interaction kind returned"),
        };
        tracing::debug!("Got /role interaction response, selected {selected}");

        let mut member = ctx.author_member().await.user_error("Not a member")?;
        tracing::debug!("Got member data for /role invoker");
        let current_color_roles = member.roles.iter().filter(|x| config.roles_color.contains(x));
        for role in current_color_roles {
            member.remove_role(&ctx.serenity_context(), role).await?;
        }
        tracing::debug!("Removed roles of user");

        let response_embed = if selected == NONE_VALUE {
            embeds::make_success_embed(ctx.serenity_context(), "Success! Removed your colorrole")
                .await
        } else {
            let role_id = selected.parse::<RoleId>().context("Invalid role")?;
            member.to_mut().add_role(&ctx.serenity_context(), role_id).await?;
            tracing::debug!("added role {} to {}", role_id, member.user.tag());

            embeds::make_success_embed(
                ctx.serenity_context(),
                &format!("Success! You're now {}", role_id.mention()),
            )
            .await
        };

        handle
            .edit(ctx, CreateReply::default().embed(response_embed))
            .await
            .context("Failed to edit message")?;
    } else {
        tracing::debug!("Role selection timed out");
        let timed_out_embed =
            embeds::make_error_embed(ctx.serenity_context(), "No role chosen").await;
        handle
            .edit(ctx, CreateReply::default().embed(timed_out_embed).components(vec![]))
            .await
            .context("Failed to send time out message")?;
    }

    Ok(())
}
