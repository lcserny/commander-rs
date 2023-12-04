use utoipa::OpenApi;

// // TODO

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::search::search_media,
    ),
    components(
        schemas(
            crate::search::MediaFileGroup,
        ),
    ),
    tags(
        (name = "search", description = "Search media API")
    )
)]
pub struct ApiDoc;