use super::*;
use itertools::Itertools;

fn combine_multitrigger_regex<'a, I: IntoIterator<Item = &'a str>>(
    words: I,
) -> Result<regex::Regex> {
    let joined_words = words
        .into_iter()
        .map(|element| regex::escape(element))
        .join("|");
    let mut regex_builder = regex::RegexBuilder::new(&format!(r"\b(?:{})\b", joined_words));
    regex_builder
        .case_insensitive(true)
        .build()
        .context("Failed to compile highlight trigger regex")
}

#[derive(Clone)]
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

    pub fn get_triggers_for_message(&self, s: &str) -> Vec<(String, Vec<UserId>)> {
        self.combined_regex
            .find_iter(s)
            .filter_map(|m| {
                let trigger = m.as_str();
                Some((
                    trigger.to_string(),
                    self.entries.get(&trigger.to_lowercase())?.clone(),
                ))
            })
            .collect()
    }

    pub fn triggers_for_user(&self, user_id: UserId) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter(move |(_, users)| users.contains(&user_id))
            .map(|(trigger, _)| trigger.as_str())
    }

    fn remove_entry(&mut self, trigger: &str, user: UserId) -> Result<()> {
        let user_list = self
            .entries
            .get_mut(&trigger.to_lowercase())
            .context("No entry with that trigger")?;
        user_list.retain(|x| x != &user);
        if user_list.is_empty() {
            self.entries.remove(&trigger.to_lowercase());
            self.combined_regex =
                combine_multitrigger_regex(self.entries.iter().map(|(x, _)| x.as_str()))?;
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
                combine_multitrigger_regex(self.entries.iter().map(|(x, _)| x.as_str()))?;
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
                combine_multitrigger_regex(self.entries.iter().map(|(x, _)| x.as_str()))?;
        }
        Ok(())
    }
}

impl Db {
    pub async fn get_highlights(&self) -> Result<HighlightsData> {
        let mut cache = self.highlight_cache.lock().await;
        if let Some(cache) = cache.as_ref() {
            Ok(cache.clone())
        } else {
            let mut conn = self.pool.acquire().await?;
            let entries = sqlx::query!("select * from highlights")
                .fetch_all(&mut conn)
                .await?
                .into_iter()
                .map(|x| (x.word, UserId::from(x.usr as u64)))
                .into_group_map();

            let highlight_data = HighlightsData::from_entries(entries.into_iter().collect())?;
            cache.replace(highlight_data.clone());
            Ok(highlight_data)
        }
    }

    pub async fn remove_highlight(&self, user: UserId, trigger: String) -> Result<()> {
        {
            let mut conn = self.pool.acquire().await?;
            let user = user.0 as i64;
            sqlx::query!(
                "delete from highlights where word=? and usr=?",
                trigger,
                user
            )
            .execute(&mut conn)
            .await?;
        }
        let mut cache = self.highlight_cache.lock().await;
        if let Some(ref mut cache) = cache.as_mut() {
            cache.remove_entry(&trigger, user)?;
        }
        Ok(())
    }

    pub async fn set_highlight(&self, user: UserId, word: String) -> Result<()> {
        {
            let mut conn = self.pool.acquire().await?;
            let user = user.0 as i64;
            sqlx::query!(
                "insert into highlights (word, usr) values (?, ?)",
                word,
                user
            )
            .execute(&mut conn)
            .await?;
        }
        let mut cache = self.highlight_cache.lock().await;
        if let Some(ref mut cache) = cache.as_mut() {
            cache.add_entry(word, user)?;
        }

        Ok(())
    }

    pub async fn rm_highlights_of(&self, user: UserId) -> Result<()> {
        {
            let mut conn = self.pool.acquire().await?;
            let user = user.0 as i64;
            sqlx::query!("delete from highlights where usr=?", user)
                .execute(&mut conn)
                .await?;
        }

        let mut cache = self.highlight_cache.lock().await;
        if let Some(ref mut cache) = cache.as_mut() {
            cache.remove_entries_of(user)?;
        }
        Ok(())
    }
}
