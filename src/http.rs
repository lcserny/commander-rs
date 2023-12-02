use std::sync::Arc;

use axum::{Router, Extension};
use eyre::Context;
use tower::ServiceBuilder;
use tower_http::{trace::TraceLayer, cors::{CorsLayer, Any}};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{config::Settings, search, error::Error, download, command, moving, rename, db::DbClient, openapi::ApiDoc};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone)]
pub struct ApiContext {
    pub settings: Arc<Settings>,
    pub db_client: DbClient,
}

pub async fn serve(settings: Arc<Settings>, db_client: DbClient) -> eyre::Result<()> {
    let port = settings.server_port;

    let app = api_router(settings.clone(), db_client.clone()).layer(
        ServiceBuilder::new()
            .layer(Extension(ApiContext { db_client, settings, }))
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

fn api_router(settings: Arc<Settings>, db_client: DbClient) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(search::router())
        .merge(download::router())
        .merge(command::router())
        .merge(moving::router())
        .merge(rename::router(settings, db_client))
        .layer(cors_layer())
}