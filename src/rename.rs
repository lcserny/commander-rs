use std::{collections::BTreeMap, sync::Arc, str::FromStr};

use async_trait::async_trait;
use axum::{Router, routing::post, Json, extract::State};
use enum_dispatch::enum_dispatch;
use serde::{Serialize, Deserialize};
use tracing::{info, warn};

use crate::{http::{self}, config::Settings, db::DbClient};

use self::{name::{BaseInfo, NameGenerator}, disk::DiskRenamer, online_cache::OnlineCacheRenamer, tmdb::{TmdbRenamer, TmdbAPI}};

mod tmdb;
pub mod online_cache;
mod disk;
pub mod name;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum MediaFileType {
    MOVIE,
    TV,
    UNKNOWN,
}

impl FromStr for MediaFileType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MOVIE" => Ok(MediaFileType::MOVIE),
            "TV" => Ok(MediaFileType::TV),
            _ => Ok(MediaFileType::UNKNOWN),
        }
    }
}

impl ToString for MediaFileType {
    fn to_string(&self) -> String {
        match self {
            MediaFileType::MOVIE => String::from("MOVIE"),
            MediaFileType::TV => String::from("TV"),
            MediaFileType::UNKNOWN => String::from("UNKNOWN"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum MediaRenameOrigin {
    DISK,
    NAME,
    CACHE,
    TMDB,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaRenameRequest {
    name: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    media_type: MediaFileType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenamedMediaOptions {
    origin: MediaRenameOrigin,
    #[serde(rename(serialize = "mediaDescriptions", deserialize = "mediaDescriptions"))]
    descriptions: Vec<MediaDescription>,
}

impl RenamedMediaOptions {
    pub fn new(origin: MediaRenameOrigin, descriptions: Vec<MediaDescription>) -> Self {
        RenamedMediaOptions { origin, descriptions }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaDescription {
    #[serde(rename(serialize = "posterUrl", deserialize = "posterUrl"))]
    poster_url: String,
    title: String,
    date: String,
    description: String,
    cast: Vec<String>,
}

#[async_trait]
#[enum_dispatch(RenamerKind)]
pub trait Renamer {
    async fn find_options(&self, base_info: &BaseInfo, media_type: MediaFileType) -> eyre::Result<Option<RenamedMediaOptions>>;
}

type RenamerOrder = usize;

#[enum_dispatch]
enum RenamerKind {
    DiskRenamer,
    OnlineCacheRenamer,
    TmdbRenamer(TmdbRenamer<TmdbAPI>),
}

struct RenamersContext {
    renamers: BTreeMap<RenamerOrder, RenamerKind>,
    generator: NameGenerator,
}

impl RenamersContext {
    fn new(settings: Arc<Settings>, db_client: DbClient) -> Self {
        let mut renamers = BTreeMap::new();
        renamers.insert(0, RenamerKind::DiskRenamer(DiskRenamer::new(settings.clone())));
        renamers.insert(1, RenamerKind::OnlineCacheRenamer(OnlineCacheRenamer::new(db_client.clone())));
        renamers.insert(2, RenamerKind::TmdbRenamer(TmdbRenamer::new(settings.clone(), TmdbAPI::new(settings.clone()), db_client)));

        RenamersContext { 
            renamers, 
            generator: NameGenerator::new(settings), 
        }
    }
}

pub fn router(settings: Arc<Settings>, db_client: DbClient) -> Router {
    Router::new().route( "/api/v1/media-renames", post(produce_renames))
        .with_state(Arc::new(RenamersContext::new(settings, db_client)))
}

async fn produce_renames(State(rename_ctx): State<Arc<RenamersContext>>, 
        Json(req): Json<MediaRenameRequest>) -> http::Result<Json<RenamedMediaOptions>> {
    info!("produce_renames request received with payload: {:?}", req);
    
    let base_info = rename_ctx.generator.generate_base_info(req.name);
    let options = produce_rename_options(base_info, &rename_ctx.renamers, req.media_type, &rename_ctx.generator).await;

    Ok(Json(options))
}

async fn produce_rename_options(base_info: BaseInfo, renamers: &BTreeMap<RenamerOrder, 
        RenamerKind>, media_type: MediaFileType, generator: &NameGenerator) -> RenamedMediaOptions {
    for (_, renamer) in renamers {
        match renamer.find_options(&base_info, media_type).await {
            Ok(found) => match found {
                Some(o) => return o,
                None => continue,
            },
            Err(e) => {
                warn!("error occurred during rename options find: {:?}", e);
                continue;
            },
        }
    }
    RenamedMediaOptions::new(MediaRenameOrigin::NAME, generator.generate_media_descriptions(vec![base_info.formatted()]))
}