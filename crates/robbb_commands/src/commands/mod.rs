use itertools::Itertools;
use poise::serenity_prelude::{Guild, Mentionable, UserId};
use poise::serenity_prelude::{Member, Permissions};
use poise::Command;
use robbb_util::abort_with;
use robbb_util::extensions::*;
use robbb_util::prelude::*;
use robbb_util::util;

pub mod errors;
pub use errors::*;

use crate::checks::PermissionLevel;

pub mod attachment_hack;
pub mod ban;
pub mod blocklist;
pub mod emojistats;
pub mod fetch;
pub mod help;
pub mod highlights;
pub mod info;
pub mod kick;
pub mod modping;
pub mod move_users;
pub mod mute;
pub mod note;
pub mod pfp;
pub mod poll;
pub mod purge;
pub mod role;
pub mod small;
pub mod tag;
pub mod top;
pub mod unban;
pub mod warn;

pub fn all_commands() -> Vec<poise::Command<UserData, Error>> {
    let mut all_commands = vec![
        // General
        pfp::pfp(),
        info::info(),
        help::help(),
        role::role(),
        poll::poll(),
        tag::tag(),
        tag::taglist(),
        modping::modping(),
        highlights::highlights(),
        small::latency(),
        small::uptime(),
        small::repo(),
        small::invite(),
        small::description(),
        small::git(),
        small::dotfiles(),
        small::version(),
        fetch::fetch(),
        fetch::set_fetch(),
        top::top(),
        move_users::move_users(),
        // Mod-only
        info::modinfo(),
        tag::settag(),
        small::restart(),
        small::say(),
        warn::warn(),
        ban::ban(),
        kick::kick(),
        ban::ban_many(),
        unban::unban(),
        emojistats::emojistats(),
        blocklist::blocklist(),
        note::note(),
        mute::mute(),
        purge::purge(),
        small::manage_commands(),
        //attachment_hack::gather_attachments(),
        // context menu
        info::menu_info(),
        ban::menu_ban(),
        warn::menu_warn(),
        mute::menu_mute(),
    ];

    poise::framework::set_qualified_names(&mut all_commands);

    for command in all_commands.iter_mut() {
        preprocess_command(command);
    }

    all_commands
}

pub fn preprocess_command(command: &mut Command<UserData, anyhow::Error>) {
    let meta = command.custom_data.downcast_ref::<CmdMeta>();
    let perms = meta.map(|m| m.perms).unwrap_or(PermissionLevel::User);
    match perms {
        PermissionLevel::Mod => {
            command.checks.push(|ctx| Box::pin(crate::checks::check_is_moderator(ctx)))
        }
        PermissionLevel::Helper => {
            command.checks.push(|ctx| Box::pin(crate::checks::check_is_helper_or_mod(ctx)))
        }
        PermissionLevel::User => {}
    };
    command.default_member_permissions = match perms {
        PermissionLevel::Mod | PermissionLevel::Helper => Permissions::ADMINISTRATOR,
        PermissionLevel::User => Permissions::USE_APPLICATION_COMMANDS,
    };
    command.category = Some(command.category.clone().unwrap_or(match perms {
        PermissionLevel::Mod | PermissionLevel::Helper => "Moderation".to_string(),
        PermissionLevel::User => "Member".to_string(),
    }));

    for subcommand in command.subcommands.iter_mut() {
        preprocess_command(subcommand);
    }
}

pub static SELECTION_EMOJI: [&str; 19] = [
    "1️⃣",
    "2️⃣",
    "3️⃣",
    "4️⃣",
    "5️⃣",
    "6️⃣",
    "7️⃣",
    "8️⃣",
    "9️⃣",
    "🔟",
    "\u{1f1e6}",
    "\u{1f1e7}",
    "\u{1f1e8}",
    "\u{1f1e9}",
    "\u{1f1f0}",
    "\u{1f1f1}",
    "\u{1f1f2}",
    "\u{1f1f3}",
    "\u{1f1f4}",
];

pub async fn member_or_self(ctx: Ctx<'_>, member: Option<Member>) -> Res<Member> {
    if let Some(member) = member {
        Ok(member)
    } else {
        Ok(ctx.author_member().await.user_error("failed to fetch message author")?.into_owned())
    }
}

pub struct CmdMeta {
    perms: PermissionLevel,
}
