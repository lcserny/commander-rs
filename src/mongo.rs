use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDateTime;

use futures::TryStreamExt;
use mongodb::{bson::{DateTime, doc, Bson, Document}, Client};
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

impl From<DownloadedMedia> for MongoDownloadedMedia {
    fn from(value: DownloadedMedia) -> Self {
        MongoDownloadedMedia { 
            file_name: value.file_name, 
            file_size: value.file_size, 
            date_downloaded: DateTime::from_millis(value.date_downloaded), 
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MongoOnlineCacheItem {
    #[serde(rename(serialize = "searchName", deserialize = "searchName"))]
    search_name: String,
    #[serde(rename(serialize = "searchYear", deserialize = "searchYear"))]
    search_year: Option<i32>,
    #[serde(rename(serialize = "coverPath", deserialize = "coverPath"))]
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
 
        let filter = doc! (
            "date_downloaded": doc! { 
                "$gte": DateTime::from_millis(date_from.timestamp_millis()), 
                "$lt": DateTime::from_millis(date_to.timestamp_millis())
            } 
        );
        let mut cursor = col.find(filter,None).await?;

        let mut all_media = vec![];
        while let Some(media) = cursor.try_next().await? {
            all_media.push(media.into());
        }

        Ok(all_media)
    }

    async fn persist(&self, items: Vec<DownloadedMedia>) -> eyre::Result<()> {
        let db = self.client.database(&self.settings.mongodb.database);
        let col = db.collection::<MongoDownloadedMedia>(&self.settings.mongodb.download_collection);

        let mongo_items: Vec<MongoDownloadedMedia> = items.into_iter()
            .map(|i| MongoDownloadedMedia::from(i))
            .collect();

        col.insert_many(mongo_items, None).await?;

        Ok(())
    }
}

fn filter_optional_eq<I: Into<Bson>>(filter: &mut Document, key: &str, val: Option<I>) {
    match val {
        Some(y) => { filter.insert(key.to_owned(), Bson::Document(doc! { "$eq": y.into() })); },
        None => (),
    }
}

#[async_trait]
impl OnlineCacheRepo for MongoDbWrapper {
    async fn retrieve_all_by_base_and_type(&self, base_info: &BaseInfo, media_type: MediaFileType) -> eyre::Result<Vec<OnlineCacheItem>> {
        let db = self.client.database(&self.settings.mongodb.database);
        let col = db.collection::<MongoOnlineCacheItem>(&self.settings.mongodb.online_collection);

        let mut filter = doc! (
            "searchName": doc! { "$eq": base_info.name() }, 
            "mediaType": doc! { "$eq": media_type } 
        );
        filter_optional_eq(&mut filter, "searchYear", base_info.year());

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