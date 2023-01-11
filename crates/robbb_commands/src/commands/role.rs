use futures::StreamExt;
use poise::serenity_prelude::{interaction::InteractionResponseType, RoleId};
use robbb_util::embeds;

use super::*;

/// Select a role. The role can be selected in a popup.
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    custom_data = "CmdMeta { perms: PermissionLevel::User }"
)]
pub async fn role(ctx: Ctx<'_>) -> Res<()> {
    const ROLE_OPTION_NONE: &str = "NONE";
    let config = ctx.get_config();

    let guild = ctx.guild().expect("guild_only");
    let available_roles = config.roles_color.iter().filter_map(|r| guild.roles.get(r));

    let handle = ctx
        .send(|m| {
            m.embed(|e| {
                e.title("Available roles");
                e.description(config.roles_color.iter().map(|r| r.mention()).join("\n"))
            });
            m.components(|c| {
                c.create_action_row(|r| {
                    r.create_select_menu(|s| {
                        s.custom_id("role").min_values(1).max_values(1);
                        s.options(|o| {
                            for role in available_roles {
                                o.create_option(|o| o.label(role.name.clone()).value(role.id));
                            }
                            o.create_option(|o| o.label("None").value(ROLE_OPTION_NONE))
                        })
                    })
                })
            });
            m.ephemeral(true)
        })
        .await?;
    let mut roles_msg = handle.message().await?;

    if let Some(interaction) = roles_msg
        .to_mut()
        .await_component_interactions(ctx.serenity_context())
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(10))
        .collect_limit(1)
        .build()
        .next()
        .await
    {
        if let Some(role_id) = interaction.data.values.first() {
            let mut member = ctx.author_member().await.user_error("Not a member")?;

            member.to_mut().remove_roles(&ctx.serenity_context(), &config.roles_color).await?;

            let response_embed = if role_id != ROLE_OPTION_NONE {
                let role_id = RoleId(role_id.parse()?);
                member.to_mut().add_role(&ctx.serenity_context(), role_id).await?;

                embeds::make_success_embed(
                    ctx.serenity_context(),
                    &format!("Success! You're now {}", role_id.mention()),
                )
                .await
            } else {
                embeds::make_success_embed(
                    ctx.serenity_context(),
                    "Success! Removed your colorrole",
                )
                .await
            };

            interaction
                .create_interaction_response(&ctx.serenity_context(), |ir| {
                    ir.kind(InteractionResponseType::UpdateMessage).interaction_response_data(|d| {
                        d.set_embed(response_embed).components(|c| c)
                    })
                })
                .await?;
        }
    } else {
        let timed_out_embed =
            embeds::make_error_embed(ctx.serenity_context(), "No role chosen").await;
        roles_msg
            .to_mut()
            .edit(&ctx.serenity_context(), |e| e.set_embed(timed_out_embed).components(|c| c))
            .await?;
    }

    Ok(())
}
