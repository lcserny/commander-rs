use std::sync::Arc;

use aho_corasick::AhoCorasick;
use async_trait::async_trait;
use eyre::eyre;
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::{config::Settings, rename::external::{ExternalSearcher, ExternalMedia}};

const SEARCH_PATS: &[&str; 4] = &["{base_url}", "{api_key}", "{query}", "{year}"];
const CREDIT_PATS: &[&str; 3] = &["{base_url}", "{id}", "{api_key}"];

#[derive(Debug, Serialize, Deserialize)]
struct MovieResults {
    page: i32,
    total_results: i64,
    total_pages: i64,
    results: Vec<Movie>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TvResults {
    page: i32,
    total_results: i64,
    total_pages: i64,
    results: Vec<Tv>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Movie {
    title: String,
    poster_path: Option<String>,
    release_date: String,
    overview: String,
    id: i32,
    #[serde(skip_deserializing)]
    cast: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Tv {
    name: String,
    poster_path: Option<String>,
    first_air_date: String,
    overview: String,
    id: i32,
    #[serde(skip_deserializing)]
    cast: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Credits {
    cast: Vec<Person>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    character: String,
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

    fn produce_url(&self, search_url: &str, year: Option<i32>, query: &str) -> eyre::Result<String> {
        let year_str = match year {
            Some(y) => y.to_string(),
            None => String::new(),
        };
        let tmdb_cfg = &self.settings.tmdb;

        let replacements: &[&str; 4] = &[&tmdb_cfg.base_url, &tmdb_cfg.api_key, query, &year_str];
        let url_builder = AhoCorasick::new(SEARCH_PATS)?;
        Ok(url_builder.replace_all(search_url, replacements))
    }

    fn produce_credits_url(&self, credits_url: &str, id: String) -> eyre::Result<String> {
        let tmdb_cfg = &self.settings.tmdb;
        let replacements: &[&str; 3] = &[&tmdb_cfg.base_url, &id, &tmdb_cfg.api_key];
        let url_builder = AhoCorasick::new(CREDIT_PATS)?;
        Ok(url_builder.replace_all(credits_url, replacements))
    }

    fn convert_tv(&self, shows: Vec<Tv>) -> Vec<ExternalMedia> {
        shows.into_iter()
            .map(|s| {
                ExternalMedia { 
                    title: s.name, 
                    poster_path: s.poster_path, 
                    date: s.first_air_date, 
                    description: s.overview, 
                    id: s.id, 
                    cast: s.cast, 
                }
            })
            .collect()
    }

    fn convert_movies(&self, movies: Vec<Movie>) -> Vec<ExternalMedia> {
        movies.into_iter()
            .map(|m| {
                ExternalMedia { 
                    title: m.title, 
                    poster_path: m.poster_path, 
                    date: m.release_date, 
                    description: m.overview, 
                    id: m.id, 
                    cast: m.cast, 
                }
            })
            .collect()
    }

    async fn get_request<M: DeserializeOwned>(&self, url: String) -> eyre::Result<M> {
        let resp = self.client.get(url).send().await?.text().await?;
        match serde_json::from_str::<M>(&resp) {
            Ok(r) => Ok(r),
            Err(e) => {
                return Err(eyre!("received response from TMDB: {:#?}, error {:?}", &resp, e));
            },
        }
    }
}

#[async_trait]
impl ExternalSearcher for TmdbAPI {
    async fn search_tv(&self, query: &str, year: Option<i32>) -> eyre::Result<Vec<ExternalMedia>> {
        let tmdb_cfg = &self.settings.tmdb;
        let url = self.produce_url(&tmdb_cfg.search_tv_url, year, query)?;
        let mut resp = self.get_request::<TvResults>(url).await?;

        for tv in &mut resp.results {
            let id = tv.id.to_string();
            let url = self.produce_credits_url(&tmdb_cfg.tv_credits_url, id)?;
            let resp = self.client.get(url).send().await?.json::<Credits>().await?;  
            tv.cast = resp.cast.into_iter().map(|p| p.character).collect();
        }
        
        Ok(self.convert_tv(resp.results))
    }

    async fn search_movie(&self, query: &str, year: Option<i32>) -> eyre::Result<Vec<ExternalMedia>> {
        let tmdb_cfg = &self.settings.tmdb;
        let url = self.produce_url(&tmdb_cfg.search_movies_url, year, query)?;
        let mut resp = self.get_request::<MovieResults>(url).await?;

        for movie in &mut resp.results {
            let id = movie.id.to_string();
            let url = self.produce_credits_url(&tmdb_cfg.movie_credits_url, id)?;
            let resp = self.client.get(url).send().await?.json::<Credits>().await?;  
            movie.cast = resp.cast.into_iter().map(|p| p.character).collect();
        }
        
        Ok(self.convert_movies(resp.results))
    }
}