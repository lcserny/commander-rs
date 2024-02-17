use async_trait::async_trait;
use chrono::{NaiveDateTime, DateTime, Utc};
use serde::Serialize;

use crate::db::DbClient;

use super::{Renamer, name::BaseInfo, RenamedMediaOptions, MediaFileType, MediaDescription, MediaRenameOrigin};

#[derive(Serialize, Debug)]
pub struct OnlineCacheItem {
    pub search_name: String,
    pub search_year: Option<i32>,
    pub cover_path: String,
    pub title: String,
    pub date: i64,
    pub description: String,
    pub cast: Vec<String>,
    pub media_type: MediaFileType,
}

#[async_trait]
pub trait OnlineCacheRepo: Send + Sync {
    async fn retrieve_all_by_base_and_type(&self, base_info: &BaseInfo, media_type: MediaFileType) -> eyre::Result<Vec<OnlineCacheItem>>;
    async fn save_items(&self, items: Vec<OnlineCacheItem>) -> eyre::Result<()>;
}

pub struct CacheRenamer {
    db_client: DbClient,
}

impl CacheRenamer {
    pub fn new(db_client: DbClient) -> Self {
        CacheRenamer { db_client }
    }
}

#[async_trait]
impl Renamer for CacheRenamer {
    async fn find_options(&self, base_info: &BaseInfo, media_type: MediaFileType) -> eyre::Result<Option<RenamedMediaOptions>> {
        let items = self.db_client.online_cache_repo()
            .retrieve_all_by_base_and_type(&base_info, media_type).await?;

        let descs: Vec<MediaDescription> = items.into_iter()
            .map(|i| MediaDescription { 
                poster_url: i.cover_path, 
                title: i.title, 
                date: to_date(i.date), 
                description: i.description, 
                cast: i.cast 
            })
            .collect();

        if descs.is_empty() {
            return Ok(None);
        }

        Ok(Some(RenamedMediaOptions::new(MediaRenameOrigin::CACHE, descs)))
    }
}

fn to_date(millis: i64) -> String {
    match NaiveDateTime::from_timestamp_millis(millis) {
        Some(n) => {
            let date_time: DateTime<Utc> = DateTime::from_naive_utc_and_offset(n, Utc);
            date_time.format("%Y-%m-%d").to_string()
        },
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::to_date;

    #[test]
    fn date_conversion() {
        let millis = 1700092800000;
        let str_date = to_date(millis);
        assert_eq!("2023-11-16".to_owned(), str_date);
    }
}