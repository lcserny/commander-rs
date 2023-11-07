use std::{path::{PathBuf, Path}, sync::Arc, ffi::OsStr};

use axum::{Router, routing::post, Json, Extension, extract::State};
use regex::Regex;
use serde::{Serialize, Deserialize};
use tracing::{warn, info};
use walkdir::DirEntry;

use crate::{search::MediaFileGroup, rename::MediaFileType, http::ApiContext, config::Settings, files};

const SUBS_DIR: &str = "Subs";
const EPISODE_SEGMENT_REGEX: &str = r".*[eE](\d{1,2}).*";
const MOVIE_EXISTS_ERROR: &str = "movie already exists";

#[derive(Debug, Serialize, Deserialize)]
struct MediaMoveReq {
    #[serde(rename(serialize = "fileGroup", deserialize = "fileGroup"))]
    file_group: MediaFileGroup,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    mtype: MediaFileType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaMoveError {
    #[serde(rename(serialize = "mediaPath", deserialize = "mediaPath"))]
    media_path: String,
    error: String,
}

#[derive(Debug)]
pub struct SubsMoveOp {
    subs_src: PathBuf,
    subs_dest: PathBuf,
    mtype: MediaFileType,
}

pub struct SubsMover {
    settings: Arc<Settings>,
    episode_regex: Arc<Regex>,
}

pub struct MediaMover {
    settings: Arc<Settings>,
    subs_mover: SubsMover,
}

impl MediaMoveError {
    fn as_vec(media_path: String, error: String) -> Vec<Self> {
        vec![MediaMoveError { media_path, error }]
    }
}

impl MediaMover {
    pub fn new(settings: Arc<Settings>, subs_mover: SubsMover) -> Self {
        MediaMover { settings, subs_mover }
    }

    // TODO: async moves and subs moves?
    pub fn move_media(&self, file_group: MediaFileGroup, mtype: MediaFileType) -> Vec<MediaMoveError> {
        let mut errors = vec![];

        if self.movie_exists(&file_group.name, mtype) {
            return MediaMoveError::as_vec(file_group.name, MOVIE_EXISTS_ERROR.to_owned());
        }

        let dest_root = match &mtype {
            MediaFileType::MOVIE => &self.settings.filesystem.movies_path,
            MediaFileType::TV => &self.settings.filesystem.tv_path,
        };

        file_group.videos
            .into_iter()
            .for_each(|video| {
                let media_src = Path::new(&file_group.path).join(&video);
                let media_dest = Path::new(dest_root).join(&file_group.name).join(&video);

                match files::move_files(&media_src, &media_dest) {
                    Ok(_) => {},
                    Err(e) => {
                        warn!("could not move media: {:?}", e);
                        errors.push(MediaMoveError { media_path: media_src.to_string_lossy().into_owned(), error: format!("{:?}", e) });
                    },
                }
            });

        let subs_src = Path::new(&file_group.path).to_path_buf();
        let subs_dest = Path::new(dest_root).join(&file_group.name);
        let subs_move_op = SubsMoveOp { subs_src, subs_dest, mtype };

        let mut subs_errors = self.subs_mover.move_subs(subs_move_op);
        errors.append(&mut subs_errors);

        if errors.is_empty() {
            return self.clean_media_src(&file_group.path);
        }

        errors
    }

    fn movie_exists(&self, media_name: &String, media_type: MediaFileType) -> bool {
        match media_type {
            MediaFileType::MOVIE => {
                let movie_path = Path::new(&self.settings.filesystem.movies_path).join(media_name);
                movie_path.exists() && movie_path.is_dir()
            },
            MediaFileType::TV => false,
        }
    }

    fn clean_media_src(&self, path_str: &String) -> Vec<MediaMoveError> {
        if &self.settings.filesystem.downloads_path == path_str
            || &self.settings.filesystem.movies_path == path_str
            || &self.settings.filesystem.tv_path == path_str {
            info!("cleaning aborted, media src dir is important folder: {:?}", path_str);
            return vec![];
        }

        let path = Path::new(path_str);
        for restricted_path in &self.settings.mv.restricted_remove_paths {
            match path.iter().last() {
                Some(last_segment) => {
                    if OsStr::new(restricted_path) == last_segment {
                        info!("clean media src dir aborted, restricted folder: {}", restricted_path);
                        return vec![];
                    }
                },
                None => (),
            }
        }

        match files::delete_dir(path) {
            Ok(_) => vec![],
            Err(e) => MediaMoveError::as_vec(path_str.to_owned(), e.to_string()),
        }
    }
}

impl SubsMover {
    pub fn new(settings: Arc<Settings>, episode_regex: Arc<Regex>) -> Self {
        SubsMover { settings, episode_regex }
    }

    pub fn move_subs(&self, op: SubsMoveOp) -> Vec<MediaMoveError> {
        if &op.subs_src.to_string_lossy() == &self.settings.filesystem.downloads_path {
            info!("path to move subs is root Downloads path, skipping operation");
            return vec![];
        }
        
        let mut subs = vec![];
        match files::walk_files(&op.subs_src, self.settings.mv.subs_max_depth) {
            Ok(mut files) => subs.append(&mut files),
            Err(e) => {
                warn!("couuld not walk path, {:?}", e);
                return MediaMoveError::as_vec(op.subs_src.to_string_lossy().into_owned(), e.to_string());
            },
        }

        subs = subs.into_iter()
            .filter(|sub| self.exclude_non_subs(sub))
            .collect();

        if subs.is_empty() {
            info!("no subs found in subs src {}", op.subs_src.to_string_lossy());
            return vec![];
        }

        match op.mtype {
            MediaFileType::MOVIE => self.move_movie_subs(op, subs),
            MediaFileType::TV => self.move_tv_subs(op, subs),
        }
    }

    fn exclude_non_subs(&self, sub: &DirEntry) -> bool {
        self.settings.mv.subs_ext.iter()
            .any(|ext| match sub.path().extension() {
                Some(sub_ext) => OsStr::new(ext) == sub_ext,
                None => false,
            })
    }

    fn move_movie_subs(&self, op: SubsMoveOp, subs: Vec<DirEntry>) -> Vec<MediaMoveError> {
        subs.into_iter()
            .map(|sub| {
                let src = sub.into_path();
                let dest = op.subs_dest.join(src.file_name().unwrap());
                match files::move_files(&src, &dest) {
                    Ok(_) => None,
                    Err(e) => {
                        warn!("could not move sub: {:?}", e);
                        Some(MediaMoveError { 
                            media_path: src.to_string_lossy().into_owned(), 
                            error: e.to_string(), 
                        })
                    },
                }
            })
            .filter(|o| o.is_some())
            .map(|o| o.unwrap())
            .collect()
    }

    fn move_tv_subs(&self, op: SubsMoveOp, subs: Vec<DirEntry>) -> Vec<MediaMoveError> {
        subs.into_iter()
            .map(|sub| {
                let src = sub.into_path();

                let mut sub_name = src.file_name().unwrap().to_string_lossy().into_owned();
                src.iter().for_each(|segment| {
                    let segment = segment.to_string_lossy().into_owned();
                    if self.episode_regex.is_match(&segment) {
                        sub_name = format!("{}.{}", segment, sub_name);
                    }
                });

                let dest = op.subs_dest.join(SUBS_DIR).join(&sub_name);

                match files::move_files(&src, &dest) {
                    Ok(_) => None,
                    Err(e) => {
                        warn!("could not move sub: {:?}", e);
                        Some(MediaMoveError { 
                            media_path: src.to_string_lossy().into_owned(), 
                            error: e.to_string(), 
                        })
                    },
                }
            })
            .filter(|o| o.is_some())
            .map(|o| o.unwrap())
            .collect()
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/api/v1/media-moves", post(move_media)
        .with_state(Arc::new(Regex::new(EPISODE_SEGMENT_REGEX).unwrap())))
}

async fn move_media(State(episode_regex): State<Arc<Regex>>, ctx: Extension<ApiContext>, Json(req): Json<MediaMoveReq>) -> Json<Vec<MediaMoveError>> {
    let subs_mover = SubsMover::new(ctx.settings.clone(), episode_regex.clone());
    let mover = MediaMover::new(ctx.settings.clone(), subs_mover);
    let errors = mover.move_media(req.file_group, req.mtype);

    Json(errors)
}