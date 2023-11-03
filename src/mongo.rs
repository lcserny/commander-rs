use std::sync::Arc;

use async_trait::async_trait;
use chrono::{NaiveDateTime, Datelike, Timelike};
use eyre::Context;
use futures::TryStreamExt;
use mongodb::{bson::{DateTime, doc}, Client};
use serde::{Serialize, Deserialize};

use crate::{download::{DownloadCacheRetriever, DownloadedMedia}, config::Settings};

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

pub struct MongoDownloadsCacheRetriever {
    client: Client,
    settings: Arc<Settings>,
}

impl MongoDownloadsCacheRetriever {
    pub fn new(client: Client, settings: Arc<Settings>) -> Self {
        MongoDownloadsCacheRetriever { client, settings }
    }
}

#[async_trait]
impl DownloadCacheRetriever for MongoDownloadsCacheRetriever {
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