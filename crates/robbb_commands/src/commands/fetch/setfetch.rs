use anyhow::Context;
use chrono::Utc;
use futures::StreamExt;
use poise::serenity_prelude::{Attachment, CollectModalInteraction, InteractionResponseType};
use poise::Modal;
use robbb_util::embeds;

use super::*;
use robbb_db::fetch::Fetch;
use robbb_util::extensions::StrExt;
use std::collections::HashMap;
use std::str::FromStr;

const SETFETCH_USAGE: &str = indoc::indoc!("
    Run this: 
    `curl -s https://raw.githubusercontent.com/unixporn/robbb/master/fetcher.sh | sh`
    and follow the instructions. It's recommended that you download and read the script before running it, 
    as piping curl to sh isn't always the safest practice. (<https://blog.dijit.sh/don-t-pipe-curl-to-bash>) 

    **NOTE**: use `/setfetch update` to manually update your fetch (including the image!).
    **NOTE**: /git, /dotfiles, and /description are different commands"
);

/// Set your fetch data
#[poise::command(
    slash_command,
    guild_only,
    category = "Miscellaneous",
    rename = "setfetch",
    subcommands("set_fetch_set", "set_fetch_update", "set_fetch_clear")
)]
pub async fn set_fetch(_ctx: Ctx<'_>) -> Res<()> {
    Ok(())
}

#[derive(Debug, poise::Modal)]
struct SetfetchModal {
    #[paragraph]
    #[placeholder = "Distro: MyCoolDistro\nKernel: Linux\nTerminal: Alacritty\nDE/WM: Kiwmi\n..."]
    #[name = "Setfetch data"]
    content: String,
}

/// Overwrite your fetch data
#[poise::command(slash_command, guild_only, category = "Miscellaneous", rename = "set")]
pub async fn set_fetch_set(app_ctx: AppCtx<'_>) -> Res<()> {
    let ctx = Ctx::Application(app_ctx);
    let db = ctx.get_db();

    let mut instructions_msg = ctx
        .send(|m| {
            m.embed(|e| e.title("Instructions").description(SETFETCH_USAGE));
            m.components(|c| {
                c.create_action_row(|c| c.create_button(|c| c.custom_id("done").label("Done!")))
            })
        })
        .await?
        .message()
        .await?;

    let old_fetch_data = db.get_fetch(ctx.author().id).await.ok().flatten();

    if let Some(interaction) = instructions_msg
        .await_component_interactions(&ctx.discord())
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(60))
        .collect_limit(1)
        .build()
        .next()
        .await
    {
        let modal_defaults = SetfetchModal {
            content: old_fetch_data
                .map(|x| fetch_to_setfetch_string(x))
                .unwrap_or_default(),
        };
        interaction
            .create_interaction_response(ctx.discord(), |ir| {
                *ir = SetfetchModal::create(Some(modal_defaults));
                ir
            })
            .await?;

        app_ctx
            .has_sent_initial_response
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // Wait for user to submit
        let response = CollectModalInteraction::new(&app_ctx.discord.shard)
            .author_id(interaction.user.id)
            .await
            .unwrap();

        // Send acknowledgement so that the pop-up is closed
        response
            .create_interaction_response(app_ctx.discord, |b| {
                b.kind(InteractionResponseType::DeferredUpdateMessage)
            })
            .await?;

        let response =
            SetfetchModal::parse(response.data.clone()).map_err(serenity::Error::Other)?;

        let success_embed = embeds::make_create_embed(&ctx.discord(), |e| {
            e.description("Updating your fetch data...")
        })
        .await;

        instructions_msg
            .edit(app_ctx.discord, |e| {
                e.set_embed(success_embed).components(|c| c)
            })
            .await?;

        let setfetch_data = parse_setfetch(response.content.lines().collect_vec())
            .user_error("Illegal format, please use `field: value` syntax.")
            .and_then(sanitize_fetch);
        let result_embed = match setfetch_data {
            Ok(setfetch_data) => {
                db.update_fetch(ctx.author().id, setfetch_data).await?;
                embeds::make_success_embed(&ctx.discord(), "Successfully updated your fetch").await
            }
            Err(user_err) => {
                embeds::make_error_embed(&ctx.discord(), &format!("{}", user_err)).await
            }
        };

        instructions_msg
            .edit(&ctx.discord(), |m| {
                m.components(|c| c).embed(|e| {
                    e.clone_from(&result_embed);
                    e
                })
            })
            .await?;
    } else {
        let timed_out_embed = embeds::make_error_embed(&ctx.discord(), "No data provided").await;
        instructions_msg
            .edit(&ctx.discord(), |e| {
                e.set_embed(timed_out_embed).components(|c| c)
            })
            .await?;
    }

    Ok(())
}

/// Update your fetch data
#[poise::command(
    slash_command,
    guild_only,
    category = "Miscellaneous",
    rename = "update"
)]
pub async fn set_fetch_update(
    ctx: Ctx<'_>,
    #[description = "Image"] image: Option<Attachment>,
    #[description = "Distro"] distro: Option<String>,
    #[description = "Kernel"] kernel: Option<String>,
    #[description = "Terminal"] terminal: Option<String>,
    #[description = "Editor"] editor: Option<String>,
    #[description = "Shell"] shell: Option<String>,
    #[description = "De_wm"] de_wm: Option<String>,
    #[description = "Bar"] bar: Option<String>,
    #[description = "Resolution"] resolution: Option<String>,
    #[description = "Display Protocol"] display_protocol: Option<String>,
    #[description = "GTK3 Theme"] gtk3_theme: Option<String>,
    #[description = "GTK icon theme"] gtk_icon_theme: Option<String>,
    #[description = "CPU"] cpu: Option<String>,
    #[description = "GPU"] gpu: Option<String>,
    #[description = "Memory"] memory: Option<String>,
) -> Res<()> {
    let image = image.map(|i| i.url);

    let memory = if let Some(memory) = memory {
        Some(
            byte_unit::Byte::from_str(&memory)
                .user_error("Malformed value provided for Memory")?
                .get_bytes()
                .to_string(),
        )
    } else {
        None
    };

    let data = maplit::hashmap! {
        FetchField::Image => image,
        FetchField::Distro => distro,
        FetchField::Kernel => kernel,
        FetchField::Terminal => terminal,
        FetchField::Editor => editor,
        FetchField::Shell => shell,
        FetchField::DEWM => de_wm,
        FetchField::Bar => bar,
        FetchField::Resolution => resolution,
        FetchField::DisplayProtocol => display_protocol,
        FetchField::GTK3 => gtk3_theme,
        FetchField::Icons => gtk_icon_theme,
        FetchField::CPU => cpu,
        FetchField::GPU => gpu,
        FetchField::Memory => memory,
    };
    let info = data
        .into_iter()
        .filter_map(|(k, v)| Some((k, v?)))
        .collect();
    let db = ctx.get_db();
    db.update_fetch(ctx.author().id, info).await?;
    ctx.say_success("Successfully updated your fetch data!")
        .await?;

    Ok(())
}

/// Clear your fetch data
#[poise::command(
    slash_command,
    guild_only,
    prefix_command,
    category = "Miscellaneous",
    rename = "clear"
)]
pub async fn set_fetch_clear(
    ctx: Ctx<'_>,
    #[description = "Field you want to clear"] field: Option<FetchField>,
) -> Res<()> {
    let db = ctx.get_db();

    if let Some(field) = field {
        let old_fetch = db.get_fetch(ctx.author().id).await?;
        if let Some(mut fetch) = old_fetch {
            fetch.info.remove(&field);
            db.set_fetch(ctx.author().id, fetch.info, Some(Utc::now()))
                .await?;
        }
        ctx.say_success(format!("Successfully cleared your {}", field))
            .await?;
    } else {
        db.set_fetch(ctx.author().id, HashMap::new(), Some(Utc::now()))
            .await?;
        ctx.say_success("Successfully cleared your fetch data!")
            .await?;
    }
    Ok(())
}

/// parse key:value formatted lines into a hashmap.
fn parse_setfetch(lines: Vec<&str>) -> anyhow::Result<HashMap<String, String>> {
    lines
        .into_iter()
        .map(|line| {
            line.split_once_at(':')
                .map(|(l, r)| (l.trim().to_string(), r.trim().to_string()))
                .filter(|(k, _)| !k.is_empty())
                .context("Malformed line")
        })
        .collect()
}

/// Sanitize field values and check validity of user-provided fetch data.
fn sanitize_fetch(fetch: HashMap<String, String>) -> Result<HashMap<FetchField, String>, UserErr> {
    let mut new: HashMap<FetchField, String> = HashMap::new();
    for (key, value) in fetch.into_iter() {
        let field = FetchField::from_str(&key)
            .map_err(|_| UserErr::Other(format!("Illegal fetch field: {}", key)))?;
        let value = match field {
            FetchField::Memory => byte_unit::Byte::from_str(&value)
                .user_error("Malformed value provided for Memory")?
                .get_bytes()
                .to_string(),

            FetchField::Image if !util::validate_url(&value) => {
                abort_with!("Malformed url provided for Image")
            }
            _ => value,
        };
        new.insert(field, value.to_string());
    }
    Ok(new)
}

fn fetch_to_setfetch_string(fetch: Fetch) -> String {
    fetch
        .info
        .into_iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .join("\n")
}
