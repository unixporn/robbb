use super::*;
use regex::Regex;

lazy_static::lazy_static! {
    static ref POLL_OPTION_START_OF_LINE_PATTERN: Regex = Regex::new(r#"\s*-|^\s*\d\.|^\s*\*"#).unwrap();
}

/// Get people to vote on your question!
#[command]
#[usage("poll <question> OR poll multi [title] <one option per line>")]
pub async fn poll(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let is_multi = args
        .single::<String>()
        .map(|x| x == "multi")
        .unwrap_or(false);
    args.restore();

    if is_multi {
        let mut lines = args.rest().lines().collect_vec();
        let title = lines.first().and_then(|line| line.strip_prefix("multi "));
        if !lines.is_empty() {
            lines.remove(0);
        }

        if lines.len() > SELECTION_EMOJI.len() || lines.len() < 2 {
            abort_with!(UserErr::Other(format!(
                "There must be between 2 and {} options",
                SELECTION_EMOJI.len()
            )))
        }

        let options = SELECTION_EMOJI
            .iter()
            .zip(lines.into_iter().map(|line| {
                POLL_OPTION_START_OF_LINE_PATTERN
                    .replace(line, "")
                    .to_string()
            }))
            .collect_vec();

        msg.channel_id
            .send_message(&ctx, |m| {
                m.embed(|e| {
                    match title {
                        Some(title) => e.title(format!("Poll: {}", title)),
                        None => e.title("Poll"),
                    };
                    for (emoji, option) in options.iter() {
                        e.field(format!("Option {}", emoji), option, false);
                    }
                    e.footer(|f| f.text(format!("from: {}", msg.author.tag())))
                });
                m.reference_message(msg);
                m.reactions(
                    options
                        .into_iter()
                        .map(|(emoji, _)| ReactionType::Unicode(emoji.to_string()))
                        .chain(std::iter::once(ReactionType::Unicode("🤷".to_string()))),
                )
            })
            .await?;
    } else {
        let question = args.remains().invalid_usage(&POLL_COMMAND_OPTIONS)?;
        if question.len() > 255 {
            abort_with!(UserErr::other("The question is too long :("))
        }
        msg.channel_id
            .send_message(&ctx, |m| {
                m.embed(|e| {
                    e.title(format!("Poll: {}", question));
                    e.footer(|f| f.text(format!("from: {}", msg.author.tag())))
                });
                m.reference_message(msg);
                m.reactions(vec![
                    ReactionType::Unicode("✅".to_string()),
                    ReactionType::Unicode("🤷".to_string()),
                    ReactionType::Unicode("❎".to_string()),
                ])
            })
            .await?;
    }

    msg.delete(&ctx).await?;

    Ok(())
}
