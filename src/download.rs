use async_trait::async_trait;
use axum::{Router, Extension, Json, routing::get, extract::Query};
use eyre::Context;
use serde::{Serialize, Deserialize};
use chrono::{Days, NaiveDateTime};
use tracing::info;

use crate::http::{ApiContext, self};

const DATE_PATTERN: &str = "%Y-%m-%d %H:%M:%S";

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
pub trait DownloadCacheRepo: Send + Sync {
    async fn retrieve_all_by_date_range(&self, date_from: NaiveDateTime, date_to: NaiveDateTime) -> eyre::Result<Vec<DownloadedMedia>>;
}

pub fn router() -> Router {
    Router::new().route("/api/v1/media-downloads", get(downloads_completed))
}

async fn downloads_completed(ctx: Extension<ApiContext>, Query(params): Query<DownloadsCompletedParams>) -> http::Result<Json<Vec<DownloadedMedia>>> {
    info!("downloads_completed request received with year {}, month {} and day {}", params.year, params.month, params.day);

    let date_str = format!("{}-{}-{} 00:00:00", params.year, params.month, params.day);
    let date_from = NaiveDateTime::parse_from_str(&date_str, DATE_PATTERN)
            .wrap_err_with(|| format!("could not create date from passed args: {:?}", params))?;
    let date_to = date_from.checked_add_days(Days::new(1)).unwrap();

    let media = ctx.db_client.download_cache_repo()
        .retrieve_all_by_date_range(date_from, date_to).await?;

    Ok(Json(media))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDateTime, Days};

    use crate::download::DATE_PATTERN;

    #[test]
    fn date_manip() {
        let date_str = format!("{}-{}-{} 00:00:00", 2023, 11, 30);
        let date_from = NaiveDateTime::parse_from_str(&date_str, DATE_PATTERN).unwrap();
        let date_to = date_from.checked_add_days(Days::new(1)).unwrap();

        assert_eq!("2023-11-30 00:00:00".to_owned(), date_from.format(DATE_PATTERN).to_string());
        assert_eq!("2023-12-01 00:00:00".to_owned(), date_to.format(DATE_PATTERN).to_string());
    }
}