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
    async fn persist(&self, items: Vec<DownloadedMedia>) -> eyre::Result<()>;
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
    use std::sync::Arc;

    use axum::{Extension, extract::Query};
    use chrono::{NaiveDateTime, Days};
    use mongodb::Client;
    use testcontainers::clients;

    use crate::{download::{DATE_PATTERN, DownloadedMedia, DownloadsCompletedParams, downloads_completed}, tests::{create_mongo_image, create_test_settings, MONGO_USER, MONGO_PASS, MONGO_PORT}, db::DbClient, mongo::MongoDbWrapper, http::ApiContext};

    #[test]
    fn date_manip() {
        let date_str = format!("{}-{}-{} 00:00:00", 2023, 11, 30);
        let date_from = NaiveDateTime::parse_from_str(&date_str, DATE_PATTERN).unwrap();
        let date_to = date_from.checked_add_days(Days::new(1)).unwrap();

        assert_eq!("2023-11-30 00:00:00".to_owned(), date_from.format(DATE_PATTERN).to_string());
        assert_eq!("2023-12-01 00:00:00".to_owned(), date_to.format(DATE_PATTERN).to_string());
    }

    #[tokio::test]
    async fn download_displays_correct_media() {
        let docker = clients::Cli::default();
        let container = docker.run(create_mongo_image());

        let mut settings = create_test_settings();
        settings.mongodb.connection_url = format!("mongodb://{}:{}@localhost:{}/?retryWrites=true&w=majority", 
            MONGO_USER, MONGO_PASS, container.get_host_port_ipv4(MONGO_PORT));
        let settings = Arc::new(settings);
        
        let mongo_client = Client::with_uri_str(&settings.mongodb.connection_url).await.unwrap();
        let db_wrapper = MongoDbWrapper::new(mongo_client, settings.clone());
        let db_client = DbClient::new(Arc::new(db_wrapper));

        let name = "hello";
        let size = 1;
        let date = NaiveDateTime::parse_from_str("2010-10-01 09:33:00", DATE_PATTERN).unwrap();
        let date_later = date.checked_add_days(Days::new(3)).unwrap();

        let media1 = DownloadedMedia { 
            file_name: name.to_owned(), 
            file_size: size, 
            date_downloaded: date.timestamp_millis(), 
        };

        let media2 = DownloadedMedia { 
            file_name: name.to_owned(), 
            file_size: size, 
            date_downloaded: date_later.timestamp_millis(),
        };

        db_client.download_cache_repo().persist(vec![media1, media2]).await.unwrap();

        let ctx = ApiContext { settings, db_client };
        let query = DownloadsCompletedParams { year: 2010, month: 10, day: 1 };
        let json_resp = downloads_completed(Extension(ctx), Query(query)).await.unwrap();
        let media = json_resp.0;

        assert_eq!(1, media.len());
        assert_eq!(name, media[0].file_name);
        assert_eq!(size, media[0].file_size);
        assert_eq!(date.timestamp_millis(), media[0].date_downloaded);
    }
}