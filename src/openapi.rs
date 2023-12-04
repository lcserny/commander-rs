use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::search::search_media,
        crate::download::downloads_completed,
        crate::command::execute_cmd,
        crate::moving::move_media,
        crate::rename::produce_renames,
    ),
    components(
        schemas(
            crate::search::MediaFileGroup,
            crate::download::DownloadedMedia,
            crate::command::CommandReq,
            crate::command::CommandResp,
            crate::moving::MediaMoveReq,
            crate::moving::MediaMoveError,
            crate::rename::MediaRenameRequest,
            crate::rename::RenamedMediaOptions,
        ),
    ),
    tags(
        (name = "search", description = "Search media API"),
        (name = "download", description = "Downloaded media API"),
        (name = "command", description = "Command execution API"),
        (name = "moving", description = "Moving media API"),
        (name = "rename", description = "Renaming media API"),
    )
)]
pub struct ApiDoc;