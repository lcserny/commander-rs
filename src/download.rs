use axum::{Router, Extension, Json, routing::get, extract::Query};
use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Serialize, Deserialize};
use tracing::info;

use crate::http::{ApiContext, self};

#[derive(Serialize, Debug)]
struct DownloadedMedia {
    id: ObjectId,
    file_name: String,
    file_size: u64,
    date_downloaded: DateTime,
}

#[derive(Debug, Deserialize)]
struct DownloadsCompletedParams {
    year: i32,
    month: i32,
    day: i32,
}

pub fn router() -> Router {
    Router::new().route("/api/v1/media-downloads", get(downloads_completed))
}

async fn downloads_completed(ctx: Extension<ApiContext>, Query(params): Query<DownloadsCompletedParams>) -> http::Result<Json<Vec<DownloadedMedia>>> {
    info!("downloads_completed request received with year {}, month {} and day {}", params.year, params.month, params.day);

    // create date from params
    // retrieve all from mongo collection download_cache by 
        // date_from >= date at start of day UTC
        // date_to < date + 1 day, at start of day UTC 

    Ok(Json(vec![]))
}