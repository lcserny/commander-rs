use async_trait::async_trait;
use axum::{Router, Extension, Json, routing::get, extract::Query};
use eyre::ContextCompat;
use serde::{Serialize, Deserialize};
use chrono::{NaiveDate, Days, NaiveDateTime, NaiveTime};
use tracing::info;

use crate::{http::{ApiContext, self}, mongo::MongoDownloadsCacheRetriever};

#[derive(Serialize, Debug)]
pub struct DownloadedMedia {
    pub file_name: String,
    pub file_size: i64,
    pub date_downloaded: i64,
}

#[derive(Debug, Deserialize)]
struct DownloadsCompletedParams {
    year: i32,
    month: u32,
    day: u32,
}

#[async_trait]
pub trait DownloadCacheRetriever {
    async fn retrieve_all_by_date_range(&self, date_from: NaiveDateTime, date_to: NaiveDateTime) -> eyre::Result<Vec<DownloadedMedia>>;
}

pub fn router() -> Router {
    Router::new().route("/api/v1/media-downloads", get(downloads_completed))
}

async fn downloads_completed(ctx: Extension<ApiContext>, Query(params): Query<DownloadsCompletedParams>) -> http::Result<Json<Vec<DownloadedMedia>>> {
    info!("downloads_completed request received with year {}, month {} and day {}", params.year, params.month, params.day);

    let date = NaiveDate::from_ymd_opt(params.year, params.month, params.day).wrap_err_with(|| format!("Could not create date from passed args: {:?}", params))?;
    let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let date_from = NaiveDateTime::new(date, time);
    let date_to = NaiveDateTime::new(date, time).checked_add_days(Days::new(1)).unwrap();

    let client = ctx.mongo_client.clone();
    let settings = ctx.settings.clone();
    let retriever = MongoDownloadsCacheRetriever::new(client, settings);
    let media = retriever.retrieve_all_by_date_range(date_from, date_to).await?;

    Ok(Json(media))
}