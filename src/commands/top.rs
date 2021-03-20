use itertools::Itertools;
use regex::Regex;

use super::*;
use std::collections::HashMap;

/// Get statistics on what the community uses.
#[command]
#[usage("top [field-name] [regex]")]
#[example("!top")]
#[example("!top Editor")]
#[example("!top Editor `n?vim`")]
pub async fn top(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = ctx.get_db().await;

    let field = args.single_quoted::<String>().ok();

    let fetches = db.get_all_fetches().await?;

    let field = field.and_then(|field| {
        super::fetch::NORMAL_FETCH_KEYS
            .iter()
            .find(|x| x.to_lowercase() == field.to_lowercase())
            .map(|x| x.to_string())
    });

    if let Some(field_name) = field {
        let value_pattern = args.single_quoted::<BacktickedString>().ok().map(|x| x.0);

        let field_value_counts = fetches
            .into_iter()
            .filter_map(|mut x| x.info.remove(&field_name))
            .filter(|x| !x.is_empty() && x != "0")
            .filter_map(|value| format_fetch_field_value(&field_name, value))
            .counts();

        if let Some(value_pattern) = value_pattern {
            let regex = Regex::new(&value_pattern).user_error("Invalid regex")?;

            let total_field_values: usize = field_value_counts.iter().map(|(_, n)| n).sum();

            let matching_value_count: usize = field_value_counts
                .iter()
                .filter(|(value, _)| regex.is_match(value))
                .map(|(_, count)| count)
                .sum();
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
        } else {
            let total_field_values: usize = field_value_counts.iter().map(|(_, n)| n).sum();

            let top_ten_field_value_counts = field_value_counts
                .into_iter()
                .sorted_by_key(|(_, cnt)| *cnt)
                .rev()
                .take(10);

            msg.reply_embed(&ctx, |e| {
                e.title(format!("Top {}", util::pluralize(&field_name)));
                e.description(
                    top_ten_field_value_counts
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
                        .join("\n"),
                );
            })
            .await?;
        }
    } else {
        let mut data = HashMap::<String, Vec<String>>::new();
        for fetch in fetches {
            for field_name in super::fetch::NORMAL_FETCH_KEYS.iter() {
                let data_value = data.entry(field_name.to_string()).or_insert_with(Vec::new);
                if let Some(field) = fetch.info.get(*field_name) {
                    data_value.push(field.clone());
                }
            }
        }
        let maxes = data.into_iter().filter_map(|(field_name, values)| {
            let values_cnt = values.len();
            let (most_popular_value, most_popular_cnt) = values
                .into_iter()
                .filter(|x| !x.is_empty() && x != "0")
                .counts()
                .into_iter()
                .max_by_key(|(_, cnt)| *cnt)?;

            let most_popular_value = format_fetch_field_value(&field_name, most_popular_value)?;

            Some((
                field_name,
                most_popular_value,
                most_popular_cnt,
                ((most_popular_cnt as f64 / values_cnt as f64) * 100f64),
            ))
        });

        msg.reply_embed(&ctx, |e| {
            e.title("Top");
            e.description(
                maxes
                    .map(|(field, value, _cnt, perc)| {
                        format!("**{}**: {} ({:.2}%)", field, value, perc)
                    })
                    .join("\n"),
            );
        })
        .await
        .context("sending reply to top-command")?;
    }
    Ok(())
}
