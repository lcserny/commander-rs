use std::sync::Arc;

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
use eyre::eyre;
use regex::Regex;
use serde::{Serialize, Deserialize};
use tracing::warn;

use crate::{db::DbClient, config::Settings};

use super::{Renamer, name::BaseInfo, RenamedMediaOptions, MediaFileType, MediaDescription, cache::OnlineCacheItem, MediaRenameOrigin};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalMedia {
    pub title: String,
    pub poster_path: Option<String>,
    pub date: String,
    pub description: String,
    pub id: i32,
    #[serde(skip_deserializing)]
    pub cast: Vec<String>,
}

#[async_trait]
pub trait ExternalSearcher: Send + Sync {
    async fn search_tv(&self, query: &str, year: Option<i32>) -> eyre::Result<Vec<ExternalMedia>>;
    async fn search_movie(&self, query: &str, year: Option<i32>) -> eyre::Result<Vec<ExternalMedia>>;
}

pub struct ExternalRenamer<S: ExternalSearcher> {
    settings: Arc<Settings>,
    searcher: S,
    db_client: DbClient,
    special_chars_regex: Regex,
}

impl <S: ExternalSearcher> ExternalRenamer<S> {
    pub fn new(settings: Arc<Settings>, searcher: S, db_client: DbClient) -> Self {
        let special_chars_regex = Regex::new(r"[^a-zA-Z0-9-\s]").unwrap();
        ExternalRenamer { settings, searcher, db_client, special_chars_regex }
    }

    fn parse_poster(&self, poster_path: String) -> String {
        match poster_path.is_empty() {
            true => poster_path,
            false => format!("{}{}", &self.settings.online.poster_base, &poster_path),
        }
    }

    fn parse_title(&self, title: String) -> String {
        let t = title.replace("&", "and");
        self.special_chars_regex.replace_all(&t, "").to_string()
    }

    fn convert_media(&self, media: Vec<ExternalMedia>) -> Vec<MediaDescription> {
        media.into_iter()
            .map(|m| MediaDescription {
                poster_url: self.parse_poster(m.poster_path.unwrap_or_default()),
                title: self.parse_title(m.title),
                date: m.date,
                description: m.description,
                cast: m.cast,
            })
            .collect()
    }

    fn create_cache_item(&self, base_info: &BaseInfo, media_desc: &MediaDescription, media_type: MediaFileType) -> OnlineCacheItem {
        OnlineCacheItem {
            search_name: base_info.name().to_owned(),
            search_year: base_info.year(),
            cover_path: media_desc.poster_url.clone(),
            title: media_desc.title.clone(),
            date: self.parse_date(&media_desc.date),
            description: media_desc.description.clone(),
            cast: media_desc.cast.clone(),
            media_type,
        }
    }

    fn parse_date(&self, date: &str) -> i64 {
        match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(d) => {
                let t = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
                NaiveDateTime::new(d, t).timestamp_millis()
            },
            Err(e) => {
                warn!("could not parse date str {:?}", e);
                0
            },
        }
    }
}

#[async_trait]
impl <S: ExternalSearcher> Renamer for ExternalRenamer<S> {
    async fn find_options(&self, base_info: &BaseInfo, media_type: MediaFileType) -> eyre::Result<Option<RenamedMediaOptions>> {
        let media_descs: Vec<MediaDescription> = match media_type {
            MediaFileType::MOVIE => self.convert_media(self.searcher.search_movie(base_info.name(), base_info.year()).await?),
            MediaFileType::TV => self.convert_media(self.searcher.search_tv(base_info.name(), base_info.year()).await?),
            MediaFileType::UNKNOWN => {
                return Err(eyre!("unknown media type provided for searcher: {:?}", media_type));
            }
        };

        if media_descs.is_empty() {
            return Ok(None);
        }

        let items: Vec<OnlineCacheItem> = media_descs.iter()
            .map(|m| self.create_cache_item(base_info, m, media_type))
            .collect();

        self.db_client.online_cache_repo().save_items(items).await?;

        Ok(Some(RenamedMediaOptions::new(MediaRenameOrigin::EXTERNAL, media_descs)))
    }
}