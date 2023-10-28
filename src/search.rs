use axum::{Router, routing::get, Extension, Json};
use serde::Serialize;
use tracing::info;

use crate::{http::{ApiContext, self}, error::Error};

#[derive(Debug, Serialize)]
struct MediaFileGroup {
    path: String,
    name: String,
    videos: Vec<String>,
}

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/media-searches", get(search_media))
}

// TODO
async fn search_media(ctx: Extension<ApiContext>) -> http::Result<Json<Vec<MediaFileGroup>>> {
    info!("search_media request received");

    Ok(Json(vec![MediaFileGroup {
        path: "myPath".to_string(),
        name: "myName".to_string(),
        videos: vec![ctx.settings.server_port.to_string()]
    }]))
    // Err(Error::Eyre)
}