use std::sync::Arc;

use axum::{Router, Extension};
use eyre::Context;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::{config::Settings, search, error::Error};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct ApiContext {
    pub settings: Arc<Settings>,
}

pub async fn serve(settings: Settings) -> eyre::Result<()> {
    let port = settings.server_port;

    let app = api_router().layer(
        ServiceBuilder::new()
            .layer(Extension(ApiContext {
                settings: Arc::new(settings),
            }))
            .layer(TraceLayer::new_for_http()),
    );

    info!("starting axum server on port {}", port);

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await
        .wrap_err_with(|| format!("could not bind port {} to axum server", port))
}

fn api_router() -> Router {
    search::router()
}