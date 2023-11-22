use std::sync::Arc;

use aho_corasick::AhoCorasick;
use async_trait::async_trait;
use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
use eyre::eyre;
use regex::Regex;
use serde::{Serialize, Deserialize};
use tracing::warn;

use crate::{db::DbClient, config::Settings};

use super::{Renamer, name::BaseInfo, RenamedMediaOptions, MediaFileType, MediaDescription, online_cache::OnlineCacheItem, MediaRenameOrigin};

const SEARCH_PATS: &[&str; 4] = &["{base_url}", "{api_key}", "{query}", "{year}"];
const CREDIT_PATS: &[&str; 3] = &["{base_url}", "{id}", "{api_key}"];

#[derive(Debug, Serialize, Deserialize)]
struct MovieResults {
    page: i32,
    total_results: i64,
    total_pages: i64,
    results: Vec<Movie>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Movie {
    pub title: String,
    pub poster_path: String,
    pub release_date: String,
    pub overview: String,
    pub id: i32,
    #[serde(skip_deserializing)]
    pub cast: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TvResults {
    page: i32,
    total_results: i64,
    total_pages: i64,
    results: Vec<Tv>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tv {
    pub name: String,
    pub poster_path: String,
    pub first_air_date: String,
    pub overview: String,
    pub id: i32,
    #[serde(skip_deserializing)]
    pub cast: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Credits {
    cast: Vec<Person>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    character: String,
}

#[async_trait]
pub trait TmdbSearcher: Send + Sync {
    async fn search_tv(&self, query: &str, year: Option<i32>) -> eyre::Result<Vec<Tv>>;
    async fn search_movie(&self, query: &str, year: Option<i32>) -> eyre::Result<Vec<Movie>>;
}

pub struct TmdbAPI {
    settings: Arc<Settings>,
    client: reqwest::Client,
}

impl TmdbAPI {
    pub fn new(settings: Arc<Settings>) -> Self {
        let client = reqwest::Client::new();
        Self { settings, client }
    }
}

#[async_trait]
impl TmdbSearcher for TmdbAPI {
    async fn search_tv(&self, query: &str, year: Option<i32>) -> eyre::Result<Vec<Tv>> {
        let year_str = match year {
            Some(y) => y.to_string(),
            None => String::new(),
        };
        let tmdb_cfg = &self.settings.tmdb;

        let replacements: &[&str; 4] = &[&tmdb_cfg.base_url, &tmdb_cfg.api_key, query, &year_str];
        let url_builder = AhoCorasick::new(SEARCH_PATS)?;
        let url = url_builder.replace_all(&tmdb_cfg.search_tv_url, replacements);

        let mut resp = self.client.get(url).send().await?.json::<TvResults>().await?;

        for tv in &mut resp.results {
            let id = tv.id.to_string();

            let replacements: &[&str; 3] = &[&tmdb_cfg.base_url, &id, &tmdb_cfg.api_key];
            let url_builder = AhoCorasick::new(CREDIT_PATS)?;
            let url = url_builder.replace_all(&tmdb_cfg.tv_credits_url, replacements);
    
            let resp = self.client.get(url).send().await?.json::<Credits>().await?;  

            tv.cast = resp.cast.into_iter().map(|p| p.character).collect();
        }
        
        Ok(resp.results)
    }

    async fn search_movie(&self, query: &str, year: Option<i32>) -> eyre::Result<Vec<Movie>> {
        let year_str = match year {
            Some(y) => y.to_string(),
            None => String::new(),
        };
        let tmdb_cfg = &self.settings.tmdb;

        let replacements: &[&str; 4] = &[&tmdb_cfg.base_url, &tmdb_cfg.api_key, query, &year_str];
        let url_builder = AhoCorasick::new(SEARCH_PATS)?;
        let url = url_builder.replace_all(&tmdb_cfg.search_movies_url, replacements);

        let mut resp = self.client.get(url).send().await?.json::<MovieResults>().await?;

        for movie in &mut resp.results {
            let id = movie.id.to_string();

            let replacements: &[&str; 3] = &[&tmdb_cfg.base_url, &id, &tmdb_cfg.api_key];
            let url_builder = AhoCorasick::new(CREDIT_PATS)?;
            let url = url_builder.replace_all(&tmdb_cfg.movie_credits_url, replacements);
    
            let resp = self.client.get(url).send().await?.json::<Credits>().await?;  

            movie.cast = resp.cast.into_iter().map(|p| p.character).collect();
        }
        
        Ok(resp.results)
    }
}

pub struct TmdbRenamer<S: TmdbSearcher> {
    settings: Arc<Settings>,
    searcher: S,
    db_client: DbClient,
    special_chars_regex: Regex,
}

impl <S: TmdbSearcher> TmdbRenamer<S> {
    pub fn new(settings: Arc<Settings>, searcher: S, db_client: DbClient) -> Self {
        let special_chars_regex = Regex::new(r"[^a-zA-Z0-9-\s]").unwrap();
        TmdbRenamer { settings, searcher, db_client, special_chars_regex }
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

    fn convert_movies(&self, movies: Vec<Movie>) -> Vec<MediaDescription> {
        movies.into_iter()
            .map(|m| MediaDescription {
                poster_url: self.parse_poster(m.poster_path),
                title: self.parse_title(m.title),
                date: m.release_date,
                description: m.overview,
                cast: m.cast,
            })
            .collect()
    }

    fn convert_tv_shows(&self, shows: Vec<Tv>) -> Vec<MediaDescription> {
        shows.into_iter()
            .map(|t| MediaDescription {
                poster_url: self.parse_poster(t.poster_path),
                title: self.parse_title(t.name),
                date: t.first_air_date,
                description: t.overview,
                cast: t.cast,
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
impl <S: TmdbSearcher> Renamer for TmdbRenamer<S> {
    async fn find_options(&self, base_info: &BaseInfo, media_type: MediaFileType) -> eyre::Result<Option<RenamedMediaOptions>> {
        let media_descs: Vec<MediaDescription> = match media_type {
            MediaFileType::MOVIE => self.convert_movies(self.searcher.search_movie(base_info.name(), base_info.year()).await?),
            MediaFileType::TV => self.convert_tv_shows(self.searcher.search_tv(base_info.name(), base_info.year()).await?),
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

        Ok(Some(RenamedMediaOptions::new(MediaRenameOrigin::TMDB, media_descs)))
    }
}