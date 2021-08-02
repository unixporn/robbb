use itertools::Itertools;
use lazy_static::lazy_static;

use crate::db::fetch::Fetch;

use super::*;
use std::collections::HashMap;

use fetch::FetchField;

/// Get statistics on what the community uses.
#[command]
#[only_in(guilds)]
#[usage("top [field-name] [`regex`]")]
#[example("!top")]
#[example("!top Editor")]
#[example("!top Editor `n?vim`")]
pub async fn top(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let fetches = db.get_all_fetches().await?;

    if !args.is_empty() {
        let field_name = args
            .single_quoted::<FetchField>()
            .user_error("Not a valid field.")?;
        if let Some(value_pattern) = args.remains() {
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
    field_name: FetchField,
    value_pattern: &str,
) -> CommandResult {
    let regex = regex::RegexBuilder::new(&value_pattern)
        .case_insensitive(true)
        .build()
        .user_error("Invalid regex")?;

    let field_values = fetches
        .into_iter()
        .filter_map(|mut x| x.info.remove(&field_name))
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
    field_name: FetchField,
) -> CommandResult {
    let field_values = fetches
        .into_iter()
        .filter_map(|mut x| x.info.remove(&field_name))
        .filter(|x| !x.is_empty() && x != "0")
        .filter_map(|value| format_fetch_field_value(&field_name, value))
        .map(|value| canonicalize_top_value(&value));

    // only compare the first word when looking at distros
    let field_value_counts = if field_name == FetchField::Distro {
        field_values
            .filter_map(|value| value.split(' ').next().map(|x| x.to_string()))
            .counts()
    } else {
        field_values.counts()
    };

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
        e.title(format!("Top {}", field_name));
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
    let mut data: HashMap<FetchField, Vec<String>> = HashMap::new();
    for fetch in fetches {
        for field_name in super::fetch::FETCH_KEY_ORDER.iter() {
            let data_value = data.entry(field_name.clone()).or_insert_with(Vec::new);
            if let Some(field) = fetch.info.get(field_name) {
                data_value.push(field.clone());
            }
        }
    }
    let maxes = data.into_iter().filter_map(|(field_name, values)| {
        let values = values.into_iter().filter(|x| !x.is_empty() && "0" != x);

        let (most_popular_value, most_popular_cnt) = values
            .clone()
            .map(|value| canonicalize_top_value(&value))
            .counts()
            .into_iter()
            .max_by_key(|(_, cnt)| *cnt)?;

        let most_popular_value = format_fetch_field_value(&field_name, most_popular_value)?;

        Some((
            field_name,
            most_popular_value,
            most_popular_cnt,
            ((most_popular_cnt as f64 / values.count() as f64) * 100f64),
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

/// Given some value that has a canonical value, return that value.
/// I.e.: "nvim" => "neovim".
fn canonicalize_top_value(value: &str) -> String {
    let value = value.trim().to_lowercase();
    EQUIVALENT_VALUES
        .iter()
        .find(|values| values.contains(&value.as_str()))
        .and_then(|x| x.first())
        .map(|x| x.to_string())
        .unwrap_or(value)
}

lazy_static! {
    static ref EQUIVALENT_VALUES: Vec<Vec<&'static str>> = vec![
        vec!["neovim", "nvim"],
        vec!["x11", "xorg"],
        vec!["arch linux", "arch"],
        vec!["visual studio code", "code", "vscode"]
    ];
}
