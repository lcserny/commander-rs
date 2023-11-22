use std::{
    collections::HashMap,
    path::{PathBuf, MAIN_SEPARATOR_STR},
    sync::Arc,
};

use axum::{routing::get, Extension, Json, Router};
use serde::{Deserialize, Serialize};
use tracing::info;
use walkdir::DirEntry;

use crate::{
    config::Settings,
    files,
    http::{self, ApiContext},
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaFileGroup {
    pub path: String,
    pub name: String,
    pub videos: Vec<String>,
}

pub struct MediaFilesParser {
    settings: Arc<Settings>,
    downloads_path: PathBuf,
}

// TODO: make it async somehow?
impl MediaFilesParser {
    fn new(settings: Arc<Settings>, downloads_path: PathBuf) -> eyre::Result<Self> {
        Ok(MediaFilesParser {
            settings,
            downloads_path,
        })
    }

    fn parse(&self, files: Vec<DirEntry>) -> Vec<DirEntry> {
        files
            .into_iter()
            .filter(|p| self.exclude_paths(p))
            .filter(|p| self.exclude_by_size(p))
            .filter(|p| self.exclude_by_content(p))
            .collect()
    }

    fn exclude_paths(&self, path: &DirEntry) -> bool {
        for exclude_path in &self.settings.search.exclude_paths {
            let path = path.path();
            if path.is_absolute() && path.to_string_lossy().contains(exclude_path) {
                return false;
            }
        }
        true
    }

    fn exclude_by_content(&self, path: &DirEntry) -> bool {
        if let Some(mime) = tree_magic_mini::from_filepath(path.path()) {
            for allowed_mime in &self.settings.search.video_mime_types {
                if allowed_mime == mime {
                    return true;
                }
            }
            if mime.starts_with("video/") {
                return true;
            }
        }
        false
    }

    fn exclude_by_size(&self, path: &DirEntry) -> bool {
        match path.metadata() {
            Ok(data) => data.len() >= self.settings.search.video_min_size_bytes,
            Err(_) => false,
        }
    }

    pub fn generate(&self, files: Vec<DirEntry>) -> Vec<MediaFileGroup> {
        let mut tmp_map = HashMap::new();

        for video_path in self.parse(files) {
            let video_path_segments: Vec<String> = video_path
                .into_path()
                .iter()
                .skip(self.downloads_path.iter().count())
                .map(|p| p.to_string_lossy().into_owned())
                .collect();

            let mut name = video_path_segments[0].clone();
            let mut path = self.downloads_path.to_path_buf();
            let mut video = name.clone();

            if video_path_segments.len() > 1 {
                path = self.downloads_path.join(&name);
                video = String::from(&video_path_segments[1..].join(MAIN_SEPARATOR_STR));
            } else {
                name = String::from(&name[..name.rfind('.').unwrap_or(name.len())]);
            }

            tmp_map
                .entry((path.to_string_lossy().into_owned(), name))
                .or_insert(vec![])
                .push(video);
        }

        tmp_map
            .into_iter()
            .map(|((path, name), videos)| MediaFileGroup { path, name, videos })
            .collect()
    }
}

pub fn router() -> Router {
    Router::new().route("/api/v1/media-searches", get(search_media))
}

pub async fn search_media(ctx: Extension<ApiContext>) -> http::Result<Json<Vec<MediaFileGroup>>> {
    info!("search_media request received");

    let settings = ctx.settings.clone();
    let downloads_path = PathBuf::from(&settings.filesystem.downloads_path);
    let files = files::walk_files(&downloads_path, settings.search.max_depth)?;
    let parser = MediaFilesParser::new(settings, downloads_path)?;

    Ok(Json(parser.generate(files)))
}
