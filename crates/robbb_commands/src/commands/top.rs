use itertools::Itertools;
use lazy_static::lazy_static;
use robbb_db::{
    fetch::Fetch,
    fetch_field::{FetchField, FETCH_KEY_ORDER},
};

use super::{fetch::format_fetch_field_value, *};
use std::collections::HashMap;

static EXCLUDED_FETCH_FIELDS: &[FetchField] =
    &[FetchField::Dotfiles, FetchField::Description, FetchField::Git, FetchField::Image];

/// Get statistics about what the community uses.
#[poise::command(
    slash_command,
    guild_only,
    custom_data = "CmdMeta { perms: PermissionLevel::User }",
    rename = "top"
)]
pub async fn top(
    ctx: Ctx<'_>,
    #[description = "What do you care about?"] field: Option<FetchField>,
    #[description = "Regex pattern for values you want more details about"] pattern: Option<String>,
) -> Res<()> {
    // defer, as get_all_fetches can apparently be quite slow
    ctx.defer().await?;
    let db = ctx.get_db();

    let fetches = db.get_all_fetches().await?;

    match (field, pattern) {
        (Some(field), Some(pattern)) => {
            top_for_regex(ctx, fetches, field, &pattern).await?;
        }
        (Some(field), None) => {
            top_for_field(ctx, fetches, field).await?;
        }
        (None, None) => {
            top_all_values(ctx, fetches).await?;
        }
        (None, Some(_)) => {
            abort_with!("You must also tell me what field to check");
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all, fields(%field_name, %value_pattern))]
async fn top_for_regex(
    ctx: Ctx<'_>,
    fetches: Vec<Fetch>,
    field_name: FetchField,
    value_pattern: &str,
) -> Res<()> {
    let regex = regex::RegexBuilder::new(value_pattern)
        .case_insensitive(true)
        .build()
        .user_error("Invalid regex")?;

    let field_values =
        fetches.into_iter().filter_map(|mut x| x.info.remove(&field_name)).collect_vec();

    let total_field_values = field_values.len();

    let matching_value_count = field_values.into_iter().filter(|x| regex.is_match(x)).count();

    let percentage = (matching_value_count as f64 / total_field_values as f64) * 100f64;

    ctx.send_embed(|e| {
        e.title(format!("Stats for matching {}s", field_name));
        e.description(indoc::formatdoc!(
            "**Matching**: `{}`
             **Total**: {}
             **Percentage**: {:.2}
            ",
            value_pattern,
            matching_value_count,
            percentage,
        ));
    })
    .await?;
    Ok(())
}

#[tracing::instrument(skip_all, fields(%field_name))]
async fn top_for_field(ctx: Ctx<'_>, fetches: Vec<Fetch>, field_name: FetchField) -> Res<()> {
    let field_values = fetches
        .into_iter()
        .filter_map(|mut x| x.info.remove(&field_name))
        .filter(|x| !x.is_empty() && x != "0")
        .filter_map(|value| format_fetch_field_value(&field_name, value))
        .map(|value| canonicalize_top_value(&value));

    // only compare the first word when looking at distros
    let field_value_counts = if field_name == FetchField::Distro {
        field_values.filter_map(|value| value.split(' ').next().map(|x| x.to_string())).counts()
    } else {
        field_values.counts()
    };

    let total_field_values: usize = field_value_counts.values().sum();

    let top_ten_field_value_counts =
        field_value_counts.into_iter().sorted_by_key(|(_, cnt)| *cnt).rev().take(10);

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

    ctx.send_embed(|e| {
        e.title(format!("Top {}", field_name));
        e.description(top_values_text);
    })
    .await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn top_all_values(ctx: Ctx<'_>, fetches: Vec<Fetch>) -> Res<()> {
    let mut data: HashMap<FetchField, Vec<String>> = HashMap::new();
    for fetch in fetches {
        for field_name in FETCH_KEY_ORDER.iter().filter(|&x| !EXCLUDED_FETCH_FIELDS.contains(x)) {
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

    ctx.send_embed(|e| {
        e.title("Top");
        e.description(top_values_text);
    })
    .await?;
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
