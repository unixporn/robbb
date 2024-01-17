use super::*;
/// Manage application commands (be careful)
///
/// Please only run this when absolutely necessary, as setting up the permissions for the commands again is pain.
#[poise::command(
    slash_command,
    custom_data = "CmdMeta { perms: PermissionLevel::Mod }",
    hide_in_help
)]
pub async fn manage_commands(ctx: Ctx<'_>) -> Res<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
