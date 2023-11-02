use std::{path::{Path, PathBuf, MAIN_SEPARATOR_STR}, collections::HashMap};

use axum::{routing::get, Extension, Json, Router};
use serde::Serialize;
use tracing::info;
use walkdir::DirEntry;

use crate::{
    files,
    http::{self, ApiContext}, config::SearchConfig,
};

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaFileGroup {
    path: String,
    name: String,
    videos: Vec<String>,
}

pub struct MediaFilesParser<'a> {
    settings: &'a SearchConfig,
    downloads_path: &'a Path,
}

impl <'a> MediaFilesParser<'a> {
    fn new(settings: &'a SearchConfig, downloads_path: &'a Path) -> eyre::Result<Self> {
        // TODO: validate input?
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
        for exclude_path in &self.settings.exclude_paths {
            let path = path.path();
            if path.is_absolute() && path.to_string_lossy().contains(exclude_path) {
                return false;
            }
        }
        true
    }
    
    fn exclude_by_content(&self, path: &DirEntry) -> bool {
        if let Some(mime) = tree_magic_mini::from_filepath(path.path()) {
            for allowed_mime in &self.settings.video_mime_types {
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
                data.len() >= self.settings.video_min_size_bytes
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

    let search_settings = &ctx.settings.search;
    let downloads_path = PathBuf::from(&ctx.settings.filesystem.downloads_path);
    let parser = MediaFilesParser::new(search_settings, &downloads_path)?;
    let files = files::walk_files(&downloads_path, search_settings.max_depth)?;

    Ok(Json(parser.generate(files)))
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, path::PathBuf, fs::{File, self}, io::BufWriter, io::Write, cmp};

    use axum::Extension;
    use rand::RngCore;

    use crate::{config::init_config, http::ApiContext};
    use super::search_media;

    fn create_file(path: PathBuf, size: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        let f = File::create(path).unwrap();
        let mut writer = BufWriter::new(f);
        
        let mut rng = rand::thread_rng();
        let mut buffer = [0; 1024];
        let mut remaining_size = size;
        
        while remaining_size > 0 {
            let to_write = cmp::min(remaining_size, buffer.len());
            let buffer=  &mut buffer[..to_write];
            rng.fill_bytes(buffer);
            writer.write(buffer).unwrap();
            
            remaining_size -= to_write;
        }
    }

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
        let cfg = init_config("config/settings_test", "TST_CMDR").unwrap();
        let fs_settings = &cfg.filesystem;
        let search_settings = &cfg.search;

        let downloads_path = PathBuf::from(&fs_settings.downloads_path);
        create_file(downloads_path.join("video1.mp4"), 6);
        create_file(downloads_path.join(&search_settings.exclude_paths[0]).join("excluded.mp4"), 6);
        create_file(downloads_path.join("video3.mkv"), 6);
        create_file(downloads_path.join("small.mp4"), 0);
        create_file(downloads_path.join("nested folder/nested.mp4"), 6);
        create_file(downloads_path.join("1/2/3/4/5/deep.mp4"), 6);

        let ctx = ApiContext { settings: Arc::new(cfg) };
        let videos_json = search_media(Extension(ctx)).await.unwrap();
        let mut videos = videos_json.0;
        videos.sort();
        
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
