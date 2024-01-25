use super::*;
use itertools::Itertools;
use std::collections::HashSet;

fn combine_multitrigger_regex<'a, I: IntoIterator<Item = &'a str>>(
    words: I,
) -> Result<regex::Regex> {
    let joined_words = words.into_iter().map(regex::escape).join("|");
    let mut regex_builder = regex::RegexBuilder::new(&format!(r"\b(?:{})\b", joined_words));
    regex_builder
        .case_insensitive(true)
        .build()
        .context("Failed to compile highlight trigger regex")
}

#[derive(Debug, Clone)]
pub struct HighlightsData {
    pub entries: HashMap<String, Vec<UserId>>,
    combined_regex: regex::Regex,
}

impl HighlightsData {
    pub fn from_entries(entries: Vec<(String, Vec<UserId>)>) -> Result<Self> {
        let triggers_regex = combine_multitrigger_regex(entries.iter().map(|(x, _)| x.as_str()))?;
        Ok(HighlightsData {
            entries: entries
                .into_iter()
                .map(|(trigger, users)| (trigger.to_lowercase(), users))
                .collect(),
            combined_regex: triggers_regex,
        })
    }

    #[tracing::instrument(skip_all, fields(msg.content = %s))]
    pub fn get_triggers_for_message(&self, s: &str) -> Vec<(String, Vec<UserId>)> {
        self.combined_regex
            .find_iter(s)
            .filter_map(|m| {
                let trigger = m.as_str();
                Some((trigger.to_string(), self.entries.get(&trigger.to_lowercase())?.clone()))
            })
            .collect()
    }

    #[tracing::instrument(skip_all)]
    pub fn triggers_for_user(&self, user_id: UserId) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter(move |(_, users)| users.contains(&user_id))
            .map(|(trigger, _)| trigger.as_str())
    }

    fn remove_entry(&mut self, trigger: &str, user: UserId) -> Result<()> {
        let user_list =
            self.entries.get_mut(&trigger.to_lowercase()).context("No entry with that trigger")?;
        user_list.retain(|x| x != &user);
        if user_list.is_empty() {
            self.entries.remove(&trigger.to_lowercase());
            self.combined_regex =
                combine_multitrigger_regex(self.entries.keys().map(|x| x.as_str()))?;
        }
        Ok(())
    }

    fn add_entry(&mut self, trigger: String, user: UserId) -> Result<()> {
        let already_in_regex = self.entries.contains_key(&trigger.to_lowercase());
        self.entries
            .entry(trigger)
            .and_modify(|f| {
                f.push(user);
            })
            .or_insert_with(|| vec![user]);
        if !already_in_regex {
            self.combined_regex =
                combine_multitrigger_regex(self.entries.keys().map(|x| x.as_str()))?;
        }
        Ok(())
    }

    fn remove_entries_of(&mut self, user: UserId) -> Result<()> {
        for users in self.entries.values_mut() {
            users.retain(|u| u != &user);
        }
        let old_length = self.entries.len();

        self.entries.retain(|_, users| !users.is_empty());

        // update the regex if some words have been removed
        if self.entries.len() != old_length {
            self.combined_regex =
                combine_multitrigger_regex(self.entries.keys().map(|x| x.as_str()))?;
        }
        Ok(())
    }
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn get_highlights(&self) -> Result<HighlightsData> {
        let mut cache = self.highlight_cache.write().await;
        if let Some(cache) = cache.as_ref() {
            Ok(cache.clone())
        } else {
            let entries = sqlx::query!("select * from highlights")
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|x| (x.word, UserId::from(x.usr as u64)))
                .into_group_map();

            let highlight_data = HighlightsData::from_entries(entries.into_iter().collect())?;
            cache.replace(highlight_data.clone());
            Ok(highlight_data)
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn remove_highlight(&self, user: UserId, trigger: String) -> Result<()> {
        {
            let user: i64 = user.into();
            sqlx::query!("delete from highlights where word=? and usr=?", trigger, user)
                .execute(&self.pool)
                .await?;
        }
        let mut cache = self.highlight_cache.write().await;
        if let Some(ref mut cache) = cache.as_mut() {
            cache.remove_entry(&trigger, user)?;
        }
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn set_highlight(&self, user: UserId, word: String) -> Result<()> {
        if BLOCKED_WORDS.contains(&unicase::Ascii::new(word.as_str())) {
            bail!(
                "Refused to set a highlight for common word {} (requested by user {})",
                word,
                user
            );
        }
        {
            let user: i64 = user.into();
            sqlx::query!("insert into highlights (word, usr) values (?, ?)", word, user)
                .execute(&self.pool)
                .await?;
        }
        let mut cache = self.highlight_cache.write().await;
        if let Some(ref mut cache) = cache.as_mut() {
            cache.add_entry(word, user)?;
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn rm_highlights_of(&self, user: UserId) -> Result<()> {
        {
            let user: i64 = user.into();
            sqlx::query!("delete from highlights where usr=?", user).execute(&self.pool).await?;
        }

        let mut cache = self.highlight_cache.write().await;
        if let Some(ref mut cache) = cache.as_mut() {
            cache.remove_entries_of(user)?;
        }
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn remove_forbidden_highlights(&self) -> Result<()> {
        let word_list =
            BLOCKED_WORDS.iter().map(|word| format!("'{}'", word.to_ascii_lowercase())).join(", ");

        // sqlx does not provide any way to properly embed a list here, so we godda use format!().
        // MAKE SURE that word_list never turns into user-provided input!
        sqlx::query(&format!("delete from highlights where lower(word) in ({})", word_list,))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

lazy_static::lazy_static! {
    static ref BLOCKED_WORDS: HashSet<unicase::Ascii<&'static str>> = include_str!("./highlights_forbidden_words").lines().map(unicase::Ascii::new).collect();
}
