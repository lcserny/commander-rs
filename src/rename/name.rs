use std::sync::Arc;

use regex::Regex;
use tracing::warn;

use crate::{config::Settings, uppercase_words};

use super::MediaDescription;

#[derive(Debug)]
pub struct BaseInfo {
    name: String,
    year: Option<i32>,
}

impl BaseInfo {
    pub fn new(name: String, year: Option<i32>) -> Self {
        BaseInfo { name, year }
    }

    pub fn formatted(&self) -> String {
        match self.year {
            Some(y) => format!("{} ({})", &self.name, y),
            None => self.name.clone(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn year(&self) -> Option<i32> {
        self.year
    }
}

#[derive(Debug)]
pub struct NameGenerator {
    title_regex: Regex,
    pre_normalize_name_regex: Regex,
    special_chars_regex: Regex,
    space_merge_regex: Regex,
    year_regex: Regex,
    name_trim_regexes: Vec<Regex>,
}

impl NameGenerator {
    pub fn new(settings: Arc<Settings>) -> Self {
        let name_trim_regexes = settings.rename.trim_regex.iter()
            .map(|r| Regex::new(r).unwrap())
            .collect();

        NameGenerator { 
            title_regex: Regex::new(r"^\s*(?<name>[a-zA-Z0-9-\s]+)\s\((?<date>(\d{4})(-\d{1,2}-\d{1,2})?)\)$").unwrap(),
            pre_normalize_name_regex: Regex::new(r"^\s*(?<name>[a-zA-Z0-9-\s]+)\s\((?<year>\d{4})(-\d{1,2}-\d{1,2})?\)$").unwrap(),
            special_chars_regex: Regex::new(r"[^a-zA-Z0-9-\s]").unwrap(),
            space_merge_regex: Regex::new(r"\s{2,}").unwrap(),
            year_regex: Regex::new(r"\s\d{4}$").unwrap(),
            name_trim_regexes,
        }
    }

    pub fn generate_base_info(&self, mut name: String) -> BaseInfo {
        match self.pre_normalize_name_regex.captures(&name) {
            Some(c) => return BaseInfo::new(c["name"].trim().to_owned(), parse_year(&c["year"])),
            None => (),
        }

        for rgx in &self.name_trim_regexes {
            name = match rgx.find(&name) {
                Some(m) => (&name[0..m.start()]).to_owned(),
                None => name,
            };
        }

        name = name.replace("&", "and");
        name = self.special_chars_regex.replace_all(&name, " ").to_string();
        name = self.space_merge_regex.replace_all(&name, " ").to_string();
        name = uppercase_words(&name.trim());

        match self.year_regex.find(&name) {
            Some(c) => {
                let start = c.start();
                BaseInfo::new((&name[0..start]).to_owned(), parse_year(&name[start + 1..]))
            },
            None => BaseInfo::new(name, None),
        }
    }

    pub fn generate_media_descriptions(&self, titles: Vec<String>) -> Vec<MediaDescription> {
        titles.into_iter()
            .map(|t| {
                let (title, date) = match self.title_regex.captures(&t) {
                    Some(c) => (c["name"].to_owned(), c["date"].to_owned()),
                    None => (t, String::new()),
                };
                MediaDescription {
                    poster_url: String::new(),
                    title,
                    date,
                    description: String::new(),
                    cast: vec![],
                }
            })
            .collect()
    }
}

fn parse_year(text: &str) -> Option<i32> {
    match text.parse::<i32>() {
        Ok(y) => Some(y),
        Err(e) => {
            warn!("could not convert year string to number: {:?}", e);
            None
        },
    }
}