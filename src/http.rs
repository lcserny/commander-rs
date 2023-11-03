use std::sync::Arc;

use axum::{Router, Extension};
use eyre::Context;
use mongodb::Client;
use tower::ServiceBuilder;
use tower_http::{trace::TraceLayer, cors::{CorsLayer, Any}};
use tracing::info;

use crate::{config::Settings, search, error::Error, download};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct ApiContext {
    pub settings: Arc<Settings>,
    pub mongo_client: Client,
}

pub async fn serve(settings: Settings) -> eyre::Result<()> {
    let port = settings.server_port;

    let app = api_router().layer(
        ServiceBuilder::new()
            .layer(Extension(ApiContext {
                mongo_client: Client::with_uri_str(&settings.mongodb.connection_url).await?,
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

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(false)
}

fn api_router() -> Router {
    search::router()
        .merge(download::router())
        // TODO: add other routers
        .layer(cors_layer())
}