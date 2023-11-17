use std::{path::{PathBuf, MAIN_SEPARATOR_STR}, collections::HashMap, sync::Arc};

use axum::{routing::get, Extension, Json, Router};
use serde::{Serialize, Deserialize};
use tracing::info;
use walkdir::DirEntry;

use crate::{
    files,
    http::{self, ApiContext}, config::Settings,
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
        Ok(MediaFilesParser { settings, downloads_path })
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
            Ok(data) => {
                data.len() >= self.settings.search.video_min_size_bytes
            },
            Err(_) => false,
        }
    }
    
    pub fn generate(&self, files: Vec<DirEntry>) -> Vec<MediaFileGroup> {
        let mut tmp_map = HashMap::new();
        
        for video_path in self.parse(files) {
            let video_path_segments: Vec<String> = video_path.into_path().iter()
                .skip(self.downloads_path.iter().count())
                .map(|p| p.to_string_lossy().into_owned()).collect();
    
            let mut name = video_path_segments[0].clone();
            let mut path = self.downloads_path.to_path_buf();
            let mut video = name.clone();
    
            if video_path_segments.len() > 1 {
                path = self.downloads_path.join(&name);
                video = String::from(&video_path_segments[1..].join(MAIN_SEPARATOR_STR));
            } else {
                name = String::from(&name[..name.rfind('.').unwrap_or(name.len())]);
            }
    
            tmp_map.entry((path.to_string_lossy().into_owned(), name))
                .or_insert(vec![])
                .push(video);
        }
    
        tmp_map.into_iter()
            .map(|((path, name), videos)| MediaFileGroup { path, name, videos })
            .collect()
    }
}

pub fn router() -> Router {
    Router::new().route("/api/v1/media-searches", get(search_media))
}

async fn search_media(ctx: Extension<ApiContext>) -> http::Result<Json<Vec<MediaFileGroup>>> {
    info!("search_media request received");

    let settings = ctx.settings.clone();
    let downloads_path = PathBuf::from(&settings.filesystem.downloads_path);
    let files = files::walk_files(&downloads_path, settings.search.max_depth)?;
    let parser = MediaFilesParser::new(settings, downloads_path)?;

    Ok(Json(parser.generate(files)))
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, path::PathBuf};

    use axum::Extension;

    use crate::{http::ApiContext, db::DbClient, tests::{EmptyDb, create_file, create_test_settings}};
    use super::search_media;

    /*
    [
      {
        "path": "/downloads/some movie folder",
        "name": "some movie folder", // this is showed in UI, used by rename
        "videos": [ // also shown in UI under, but you can't change these individually
          "video1.mp4"
        ]
      },
      {
        "path": "/downloads/some tv folder",
        "name": "some tv folder",
        "videos": [ // used by move, just resolve <path> to them
          "video1.mp4",
          "video2.mp4",
          "video3.mp4",
        ]
      },
      {
        "path": "/downloads/some nested folder", // easier to delete
        "name": "some nested folder", // notice the nested structure
        "videos": [
          "another folder/video1.mp4",
          "another folder/video2.mp4"
        ]
      },
      {
        "path": "/downloads", // notice no parent folder
        "name": "video5", // notice its generated from file name without extension
        "videos": [
          "video5.mp4",
        ]
      },
    ]
    */
    #[tokio::test]
    async fn check_search_finds_correct_media() {
        let settings = create_test_settings();
        let db_client = DbClient::new(Arc::new(EmptyDb));

        let downloads_path = PathBuf::from(&settings.filesystem.downloads_path);
        create_file(downloads_path.join("video1.mp4"), 6);
        create_file(downloads_path.join(&settings.search.exclude_paths[0]).join("excluded.mp4"), 6);
        create_file(downloads_path.join("video3.mkv"), 6);
        create_file(downloads_path.join("small.mp4"), 0);
        create_file(downloads_path.join("nested folder/nested.mp4"), 6);
        create_file(downloads_path.join("1/2/3/4/5/deep.mp4"), 6);

        let mut videos_json = search_media(Extension(ApiContext{ settings, db_client })).await.unwrap();

        videos_json.0.sort();
        
        let videos = videos_json.0;
        let downloads_str = downloads_path.to_string_lossy().into_owned();

        assert_eq!(3, videos.len());
        assert_eq!(downloads_str, videos[0].path);
        assert_eq!("video1.mp4", &videos[0].videos[0]);
        assert_eq!(downloads_str, videos[1].path);
        assert_eq!("video3.mkv", &videos[1].videos[0]);
        assert!(videos[2].path.contains("nested folder"));
        assert_eq!("nested folder", &videos[2].name);
        assert_eq!("nested.mp4", &videos[2].videos[0]);
    }
}
