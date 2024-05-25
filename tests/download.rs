#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{extract::Query, Extension};
    use chrono::{Days, NaiveDateTime};
    use commander::{
        db::DbClient,
        download::{downloads_completed, DownloadedMedia, DownloadsCompletedParams, DATE_PATTERN},
        http::ApiContext,
        mongo::MongoDbWrapper,
        tests::{create_mongo_image, create_test_settings, MONGO_PASS, MONGO_PORT, MONGO_USER},
    };
    use mongodb::Client;
    use testcontainers::clients;

    #[tokio::test]
    async fn download_displays_correct_media() {
        let docker = clients::Cli::default();
        let container = docker.run(create_mongo_image());

        let mut settings = create_test_settings();
        settings.mongodb.connection_url = format!("mongodb://{}:{}@localhost:{}/?retryWrites=true&w=majority",
            MONGO_USER, MONGO_PASS, container.get_host_port_ipv4(MONGO_PORT)
        );
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

        db_client
            .download_cache_repo()
            .persist(vec![media1, media2])
            .await
            .unwrap();

        let ctx = ApiContext {
            settings,
            db_client,
        };
        let query = DownloadsCompletedParams {
            year: 2010,
            month: 10,
            day: 1,
        };
        let json_resp = downloads_completed(Extension(ctx), Query(query))
            .await
            .unwrap();
        let media = json_resp.0;

        assert_eq!(1, media.len());
        assert_eq!(name, media[0].file_name);
        assert_eq!(size, media[0].file_size);
        assert_eq!(date.timestamp_millis(), media[0].date_downloaded);
    }
}
