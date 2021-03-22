use itertools::Itertools;
use regex::Regex;

use crate::db::fetch::Fetch;

use super::*;
use std::collections::HashMap;

/// Get statistics on what the community uses.
#[command]
#[usage("top [field-name] [`regex`]")]
#[example("!top")]
#[example("!top Editor")]
#[example("!top Editor `n?vim`")]
pub async fn top(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let field_name = args.single_quoted::<String>().ok();

    let fetches = db.get_all_fetches().await?;

    if let Some(field_name) = field_name {
        let field_name = super::fetch::find_fetch_key_matching(&field_name)
            .user_error("Not a valid fetch field")?;

        let value_pattern = args.single_quoted::<String>().ok();

        if let Some(value_pattern) = value_pattern {
            let value_pattern = util::parse_backticked_string(&value_pattern)
                .user_error("Must be surrounded in \\`backticks\\`")?;

            top_for_regex(ctx, msg, fetches, field_name, value_pattern).await?;
        } else {
            top_for_field(ctx, msg, fetches, field_name).await?;
        }
    } else {
        top_all_values(ctx, msg, fetches).await?;
    }
    Ok(())
}

async fn top_for_regex(
    ctx: &client::Context,
    msg: &Message,
    fetches: Vec<Fetch>,
    field_name: &str,
    value_pattern: &str,
) -> CommandResult {
    let regex = Regex::new(&value_pattern).user_error("Invalid regex")?;

    let field_values = fetches
        .into_iter()
        .filter_map(|mut x| x.info.remove(field_name))
        .collect_vec();

    let total_field_values = field_values.len();

    let matching_value_count = field_values
        .into_iter()
        .filter(|x| regex.is_match(x))
        .count();

    let percentage = (matching_value_count as f64 / total_field_values as f64) * 100f64;

    msg.reply_embed(&ctx, |e| {
        e.title(format!("Stats for matching {}s", field_name));
        e.description(indoc::formatdoc!(
            "**Total**: {}
             **Percentage**: {:.2}
            ",
            matching_value_count,
            percentage,
        ));
    })
    .await?;
    Ok(())
}

async fn top_for_field(
    ctx: &client::Context,
    msg: &Message,
    fetches: Vec<Fetch>,
    field_name: &str,
) -> CommandResult {
    let field_value_counts = fetches
        .into_iter()
        .filter_map(|mut x| x.info.remove(field_name))
        .filter(|x| !x.is_empty() && x != "0")
        .filter_map(|value| format_fetch_field_value(field_name, value))
        .counts();

    let total_field_values: usize = field_value_counts.iter().map(|(_, n)| n).sum();

    let top_ten_field_value_counts = field_value_counts
        .into_iter()
        .sorted_by_key(|(_, cnt)| *cnt)
        .rev()
        .take(10);

    let top_values_text = top_ten_field_value_counts
        .enumerate()
        .map(|(i, (value, count))| {
            format!(
                "**{}**. {} ({}, {:.2}%)",
                i,
                value,
                count,
                (count as f64 / total_field_values as f64) * 100f64
            )
        })
        .join("\n");

    msg.reply_embed(&ctx, |e| {
        e.title(format!("Top {}", util::pluralize(&field_name)));
        e.description(top_values_text);
    })
    .await?;
    Ok(())
}

async fn top_all_values(
    ctx: &client::Context,
    msg: &Message,
    fetches: Vec<Fetch>,
) -> CommandResult {
    let mut data = HashMap::<String, Vec<String>>::new();
    for fetch in fetches {
        for field_name in super::fetch::NORMAL_FETCH_KEYS.iter() {
            let data_value = data.entry(field_name.to_string()).or_insert_with(Vec::new);
            if let Some(field) = fetch.info.get(*field_name) {
                data_value.push(field.clone());
            }
        }
    }
    let maxes = data
        .into_iter()
        .sorted()
        .filter_map(|(field_name, values)| {
            let (most_popular_value, most_popular_cnt) = values
                .iter()
                .filter(|x| !x.is_empty() && x != &"0")
                .counts()
                .into_iter()
                .max_by_key(|(_, cnt)| *cnt)?;

            let most_popular_value =
                format_fetch_field_value(&field_name, most_popular_value.to_string())?;

            Some((
                field_name,
                most_popular_value,
                most_popular_cnt,
                ((most_popular_cnt as f64 / values.len() as f64) * 100f64),
            ))
        });

    let top_values_text = maxes
        .map(|(field, value, _cnt, perc)| format!("**{}**: {} ({:.2}%)", field, value, perc))
        .join("\n");

    msg.reply_embed(&ctx, |e| {
        e.title("Top");
        e.description(top_values_text);
    })
    .await
    .context("sending reply to top-command")?;
    Ok(())
}
