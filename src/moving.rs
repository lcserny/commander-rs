use std::{ffi::OsStr, path::Path, sync::Arc};

use axum::{extract::State, routing::post, Extension, Json, Router};

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use walkdir::DirEntry;

use crate::{
    config::Settings, files, http::ApiContext, rename::MediaFileType, search::MediaFileGroup,
};

const SUBS_DIR: &str = "Subs";
const EPISODE_SEGMENT_REGEX: &str = r".*[eE](\d{1,2}).*";

#[derive(Debug, Serialize, Deserialize)]
struct MediaMoveReq {
    #[serde(rename(serialize = "fileGroup", deserialize = "fileGroup"))]
    file_group: MediaFileGroup,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    media_type: MediaFileType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaMoveError {
    #[serde(rename(serialize = "mediaPath", deserialize = "mediaPath"))]
    media_path: String,
    error: String,
}

impl MediaMoveError {
    fn new(media_path: String, error: eyre::Report) -> Self {
        MediaMoveError { media_path, error: error.to_string(), }
    }
}

pub fn router() -> Router {
    Router::new().route( "/api/v1/media-moves", post(move_media)
        .with_state(Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap())),
    )
}

async fn move_media( State(episode_regex): State<Arc<Regex>>, ctx: Extension<ApiContext>, Json(req): Json<MediaMoveReq>) -> Json<Vec<MediaMoveError>> {
    info!("move_media request received with paylod: {:?}", req);

    let settings = ctx.settings.clone();
    let file_group = req.file_group;
    let media_path = file_group.path.clone();

    let res = match req.media_type {
        MediaFileType::MOVIE => move_media_and_subs(MovieMedia::new(settings, file_group)),
        MediaFileType::TV => move_media_and_subs(TvMedia::new(settings, file_group, episode_regex)),
        MediaFileType::UNKNOWN => {
            warn!("unknown media type provided for media {:?}", file_group);
            return Json(vec![]);
        },
    }; 
    
    let mut errors = vec![];
    if let Err(e) = res {
        errors.push(MediaMoveError::new(media_path, e));
    };

    Json(errors)
}

trait Media {
    fn already_exists(&self) -> bool;
    fn dest_root(&self) -> &str;
    fn file_group(&self) -> &MediaFileGroup;
    fn settings(&self) -> &Settings;
    fn move_subs(&self, dest: &Path, subs: Vec<DirEntry>) -> eyre::Result<()>;
}

struct MovieMedia {
    settings: Arc<Settings>,
    file_group: MediaFileGroup,
}

impl MovieMedia {
    fn new(settings: Arc<Settings>, file_group: MediaFileGroup) -> Self {
        MovieMedia { settings, file_group, }
    }
}

impl Media for MovieMedia {
    fn already_exists(&self) -> bool {
        let movie_path = Path::new(&self.settings.filesystem.movies_path).join(&self.file_group.name);
        movie_path.exists() && movie_path.is_dir()
    }

    fn dest_root(&self) -> &str {
        &self.settings.filesystem.movies_path
    }

    fn file_group(&self) -> &MediaFileGroup {
        &self.file_group
    }

    fn settings(&self) -> &Settings {
        &self.settings
    }

    fn move_subs(&self, dest: &Path, subs: Vec<DirEntry>) -> eyre::Result<()> {
        subs.iter()
            .map(|sub| {
                let src = sub.path();
                let dest = dest.join(src.file_name().unwrap());
                files::move_files(src, &dest)
            })
            .collect()
    }
}

struct TvMedia {
    settings: Arc<Settings>,
    file_group: MediaFileGroup,
    episode_regex: Arc<Regex>,
}

impl TvMedia {
    fn new(settings: Arc<Settings>, file_group: MediaFileGroup, episode_regex: Arc<Regex>) -> Self {
        TvMedia { settings, file_group, episode_regex, }
    }
}

impl Media for TvMedia {
    fn already_exists(&self) -> bool {
        false
    }

    fn dest_root(&self) -> &str {
        &self.settings.filesystem.tv_path
    }

    fn file_group(&self) -> &MediaFileGroup {
        &self.file_group
    }

    fn settings(&self) -> &Settings {
        &self.settings
    }

    fn move_subs(&self, dest: &Path, subs: Vec<DirEntry>) -> eyre::Result<()> {
        subs.iter()
            .map(|sub| {
                let src = sub.path();

                let mut sub_name = src.file_name().unwrap().to_string_lossy().into_owned();
                for segment in src.iter() {
                    let segment = segment.to_string_lossy().into_owned();
                    if self.episode_regex.is_match(&segment) {
                        sub_name = format!("{}.{}", segment, sub_name);
                        break;
                    }
                }

                let new_dest = dest.join(SUBS_DIR).join(&sub_name);
                files::move_files(src, &new_dest)
            })
            .collect()
    }
}

fn move_media_and_subs<M: Media>(media: M) -> eyre::Result<()> {
    if media.already_exists() {
        info!("media with path already exists: {}", &media.file_group().path);
        return Ok(());
    }

    let dest_root = media.dest_root();

    for video in &media.file_group().videos {
        let media_src = Path::new(&media.file_group().path).join(video);
        let media_dest = Path::new(dest_root).join(&media.file_group().name).join(video);
        files::move_files(&media_src, &media_dest)?;
    }

    let subs_src = Path::new(&media.file_group().path).to_path_buf();
    let subs_src_str = subs_src.to_string_lossy();

    if &subs_src_str == &media.settings().filesystem.downloads_path {
        info!("path to move subs is root Downloads path, skipping operation");
        return Ok(());
    }

    let mut subs = files::walk_files(&subs_src, media.settings().mv.subs_max_depth)?;
    subs = subs
        .into_iter()
        .filter(|sub| exclude_non_subs(media.settings(), sub))
        .collect();

    if subs.is_empty() {
        info!("no subs found in subs src {}", &subs_src_str);
        return Ok(());
    }

    let subs_dest = Path::new(dest_root).join(&media.file_group().name);
    media.move_subs(&subs_dest, subs)?;

    clean_media_src(media.settings(), &media.file_group().path)?;

    Ok(())
}

fn exclude_non_subs(settings: &Settings, sub: &DirEntry) -> bool {
    settings.mv.subs_ext.iter().any(|ext| 
        match sub.path().extension() {
            Some(sub_ext) => OsStr::new(ext) == sub_ext,
            None => false,
        })
}

fn clean_media_src(settings: &Settings, path_str: &str) -> eyre::Result<()> {
    if &settings.filesystem.downloads_path == path_str
        || &settings.filesystem.movies_path == path_str
        || &settings.filesystem.tv_path == path_str
    {
        info!("cleaning aborted, media src dir is important folder: {}", path_str);
        return Ok(());
    }

    let path = Path::new(path_str);
    for restricted_path in &settings.mv.restricted_remove_paths {
        match path.iter().last() {
            Some(last_segment) => {
                if OsStr::new(restricted_path) == last_segment {
                    info!( "clean media src dir aborted, restricted folder: {}", restricted_path);
                    return Ok(());
                }
            }
            None => (),
        }
    }

    files::delete_dir(path)
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn some_test() {
        todo!()
    }
}
