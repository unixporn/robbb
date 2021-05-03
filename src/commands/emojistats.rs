use super::*;

enum DisplayMode {
    InText,
    Reactions,
    Both,
}

enum OrderingMode<'a> {
    Top,
    Bottom,
    Singular(&'a EmojiData),
}

use crate::db::emoji_logging::EmojiData;

#[command]
#[usage("emojistats [in_text or reactions]")]
#[usage("emojistats [emoji] ")]
#[usage("emojistats [emoji] [in_text or reactions]")]
pub async fn emojistats(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db: std::sync::Arc<Db> = ctx.get_db().await;

    let emojis = db.get_all_emojis().await?.collect_vec();

    let mut display_mode: Option<DisplayMode> = None;
    let mut ordering_mode: Option<OrderingMode> = None;

    for arg in args.iter::<String>() {
        match arg.ok() {
            Some(x) if x == "in_text" => display_mode = Some(DisplayMode::InText),
            Some(x) if x == "reactions" => display_mode = Some(DisplayMode::Reactions),
            Some(x) if x == "both" => display_mode = Some(DisplayMode::Both),
            Some(x) if x == "top" => ordering_mode = Some(OrderingMode::Top),
            Some(x) if x == "bottom" => ordering_mode = Some(OrderingMode::Bottom),
            Some(x) if !crate::util::find_emojis(&x).is_empty() => {
                ordering_mode = Some(OrderingMode::Singular(
                    emojis
                        .iter()
                        .find(|y| {
                            crate::util::find_emojis(&x)
                                .first()
                                .map(|x| x == &y.emoji)
                                .unwrap_or(false)
                        })
                        .user_error("Could not find emoji")?,
                ))
            }
            Some(x) => {
                ordering_mode = Some(OrderingMode::Singular(
                    emojis
                        .iter()
                        .find(|e| e.emoji.name == x)
                        .user_error("Could not find emoji of that name")?,
                ))
            }
            None => abort_with!("invalid arguments"),
        }
    }

    let ordering_mode = ordering_mode.unwrap_or(OrderingMode::Top);
    let display_mode = display_mode.unwrap_or(DisplayMode::Both);

    match ordering_mode {
        OrderingMode::Singular(emoji_data) => {
            msg.reply_embed(ctx, |e| {
                e.title(format!("Emoji usage for {}", emoji_data.emoji.name));
                use DisplayMode::*;
                match display_mode {
                    Both => e.description(format!(
                        "Both reactions and in text usage: {}",
                        emoji_data.reactions + emoji_data.in_text
                    )),
                    Reactions => {
                        e.description(format!("Usage in reactions {}", emoji_data.reactions))
                    }
                    InText => e.description(format!("Usage in text {}", emoji_data.in_text)),
                };
            })
            .await?;
        }
        OrderingMode::Top => {
            let (sorted, emojis) = sort_emojis(&display_mode, emojis.into_iter());
            msg.reply_embed(ctx, |e| {
                e.title(format!(
                    "Usage sorted from top to bottom based on {} ",
                    sorted
                ));
                e.description(display_emojis(emojis, &display_mode));
            })
            .await?;
        }
        OrderingMode::Bottom => {
            let (sorted, emojis) = sort_emojis(&display_mode, emojis.into_iter());
            msg.reply_embed(ctx, |e| {
                e.title(format!(
                    "Usage sorted from bottom to top based on {}",
                    sorted
                ));
                e.description(display_emojis(emojis.rev(), &display_mode));
            })
            .await?;
        }
    }
    Ok(())
}
fn sort_emojis(
    mode: &DisplayMode,
    emojis: impl DoubleEndedIterator<Item = EmojiData>,
) -> (&str, impl DoubleEndedIterator<Item = EmojiData>) {
    use DisplayMode::*;
    match mode {
        InText => (
            "in text usage",
            emojis.sorted_by(|a, b| (a.in_text).cmp(&b.in_text)),
        ),
        Reactions => (
            "reactions",
            emojis.sorted_by(|a, b| (a.reactions).cmp(&b.reactions)),
        ),
        Both => (
            "both reactions and in text usage",
            emojis.sorted_by(|a, b| (a.in_text + a.reactions).cmp(&(b.in_text + b.reactions))),
        ),
    }
}

fn display_emojis(emojis: impl Iterator<Item = EmojiData>, mode: &DisplayMode) -> String {
    use DisplayMode::*;

    emojis
        .map(|d| {
            format!(
                "**{}** animated: {}    ***{}***",
                d.emoji.name,
                d.emoji.animated,
                match mode {
                    Both => d.reactions + d.in_text,
                    InText => d.in_text,
                    Reactions => d.reactions,
                }
            )
        })
        .join("\n")
}

//fn sort_emojis(
//    &mut args: Args,
//    emojis: impl Iterator<Item = EmojiData>,
//) -> Result<(DisplayMode, OrderingMode, impl Iterator<Item = EmojiData>)> {
//    let mode = args.single_quoted::<String>().ok();
//
//    let reactions_or_in_text = args.single_quoted::<String>().ok();
//
//    //    let part1 =match mode {
//    //        Some(x) if x == "in_text" => return Ok((DisplayMode::InText,OrderingMode::Top,emojis.sorted_by(|a, b| (a.in_text).cmp(&b.in_text)))),
//    //        Some(x) if x == "reactions" => emojis.sorted_by(|a, b| (a.reactions).cmp(&b.reactions)),
//    //        Some(x) if x == "top" => {
//    //            emojis.sorted_by(|a, b| (a.reactions + a.in_text).cmp(&(b.reactions + b.in_text)))
//    //        }
//    //        Some(x) if x == "bottom" => emojis
//    //            .sorted_by(|a, b| (a.reactions + a.in_text).cmp(&(b.reactions + b.in_text)))
//    //            .rev(),
//    //        Some(x) => {
//    //                let emoji  = crate::util::find_emojis(x).first()?;
//    //
//    //            }
//    //        _ => emojis.sorted_by(|a, b| (a.reactions + a.in_text).cmp(&(b.reactions + b.in_text))),
//    //    }
//}
