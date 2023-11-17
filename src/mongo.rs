use std::sync::Arc;

use async_trait::async_trait;
use chrono::{NaiveDateTime, Datelike, Timelike};
use eyre::Context;
use futures::TryStreamExt;
use mongodb::{bson::{DateTime, doc, Bson}, Client};
use serde::{Serialize, Deserialize};

use crate::{download::{DownloadCacheRepo, DownloadedMedia}, config::Settings, rename::{online_cache::{OnlineCacheRepo, OnlineCacheItem}, MediaFileType, name::BaseInfo}};

#[derive(Debug, Serialize, Deserialize)]
struct MongoDownloadedMedia {
    file_name: String,
    file_size: i64,
    date_downloaded: DateTime,
}

impl Into<DownloadedMedia> for MongoDownloadedMedia {
    fn into(self) -> DownloadedMedia {
        DownloadedMedia { 
            file_name: self.file_name, 
            file_size: self.file_size, 
            date_downloaded: self.date_downloaded.timestamp_millis() 
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MongoOnlineCacheItem {
    #[serde(rename(serialize = "searchName", deserialize = "searchName"))]
    search_name: String,
    #[serde(rename(serialize = "searchYear", deserialize = "searchName"))]
    search_year: Option<i32>,
    #[serde(rename(serialize = "coverPath", deserialize = "searchName"))]
    cover_path: String,
    title: String,
    date: DateTime,
    description: String,
    cast: Vec<String>,
    #[serde(rename(serialize = "mediaType", deserialize = "mediaType"))]
    media_type: String,
}

impl Into<OnlineCacheItem> for MongoOnlineCacheItem {
    fn into(self) -> OnlineCacheItem {
        OnlineCacheItem { 
            search_name: self.search_name, 
            search_year: self.search_year,
            cover_path: self.cover_path, 
            title: self.title, 
            date: self.date.timestamp_millis(), 
            description: self.description, 
            cast: self.cast, 
            media_type: self.media_type.parse::<MediaFileType>().unwrap()
        }
    }
}

impl From<OnlineCacheItem> for MongoOnlineCacheItem {
    fn from(i: OnlineCacheItem) -> Self {
        MongoOnlineCacheItem {
            search_name: i.search_name,
            search_year: i.search_year,
            cover_path: i.cover_path,
            title: i.title,
            date: DateTime::from_millis(i.date),
            description: i.description,
            cast: i.cast,
            media_type: i.media_type.to_string(),
        }
    }
}

impl From<MediaFileType> for Bson {
    fn from(value: MediaFileType) -> Self {
        Bson::String(value.to_string())
    }
}

#[derive(Clone)]
pub struct MongoDbWrapper {
    client: Client,
    settings: Arc<Settings>,
}

impl MongoDbWrapper {
    pub fn new(client: Client, settings: Arc<Settings>) -> Self {
        MongoDbWrapper { client, settings }
    }
}

#[async_trait]
impl DownloadCacheRepo for MongoDbWrapper {
    async fn retrieve_all_by_date_range(&self, date_from: NaiveDateTime, date_to: NaiveDateTime) -> eyre::Result<Vec<DownloadedMedia>> {
        let db = self.client.database(&self.settings.mongodb.database);
        let col = db.collection::<MongoDownloadedMedia>(&self.settings.mongodb.download_collection);
 
        let filter = doc! ("date_downloaded": doc! { "$gte": convert_date(date_from)?, "$lt": convert_date(date_to)?} );
        let mut cursor = col.find(filter,None).await?;

        let mut all_media = vec![];
        while let Some(media) = cursor.try_next().await? {
            all_media.push(media.into());
        }

        Ok(all_media)
    }
}

#[async_trait]
impl OnlineCacheRepo for MongoDbWrapper {
    async fn retrieve_all_by_base_and_type(&self, base_info: &BaseInfo, media_type: MediaFileType) -> eyre::Result<Vec<OnlineCacheItem>> {
        let db = self.client.database(&self.settings.mongodb.database);
        let col = db.collection::<MongoOnlineCacheItem>(&self.settings.mongodb.online_collection);

        let filter = doc! (
            "searchName": doc! { "$eq": base_info.name()}, 
            // TODO: is this filtering ok with Option<>? seems not, appears as searchYear: null,
            "searchYear": doc! { "$eq": base_info.year()}, 
            "mediaType": doc! { "$eq": media_type} 
        );
        let mut cursor = col.find(filter,None).await?;

        let mut all_media = vec![];
        while let Some(media) = cursor.try_next().await? {
            all_media.push(media.into());
        }

        Ok(all_media)
    }

    async fn save_items(&self, items: Vec<OnlineCacheItem>) -> eyre::Result<()> {
        let db = self.client.database(&self.settings.mongodb.database);
        let col = db.collection::<MongoOnlineCacheItem>(&self.settings.mongodb.online_collection);

        let docs: Vec<MongoOnlineCacheItem> = items.into_iter()
            .map(|i| MongoOnlineCacheItem::from(i))
            .collect();

        col.insert_many(docs,None).await?;

        Ok(())
    }
}

fn convert_date(date_time: NaiveDateTime) -> eyre::Result<DateTime> {
    DateTime::builder()
        .year(date_time.year())
        .month(date_time.month() as u8)
        .day(date_time.day() as u8)
        .hour(date_time.hour() as u8)
        .minute(date_time.minute() as u8)
        .second(date_time.second() as u8)
        .build()
        .wrap_err_with(|| format!("Could not convert to Bson DateTime from {:?}", date_time))
}