use std::{sync::Arc, path::Path};

use async_trait::async_trait;
use edit_distance::edit_distance;
use eyre::eyre;
use regex::Regex;
use tracing::{warn, info};
use walkdir::DirEntry;

use crate::{config::Settings, files};

use super::{Renamer, RenamedMediaOptions, name::{BaseInfo, NameGenerator}, MediaFileType, MediaRenameOrigin};

struct DiskPath {
    file_name: String,
    trimmed_file_name: String,
    similarity: usize,
}

impl DiskPath {
    fn new(entry: DirEntry, name: &str, release_date_regex: &Regex) -> Self {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let trimmed_file_name = release_date_regex.replace_all(&file_name, "").into_owned();
        let similarity = edit_distance(&trimmed_file_name, name);

        DiskPath {
            file_name,
            trimmed_file_name,
            similarity,
        }
    }
}

#[derive(Debug)]
pub struct DiskRenamer {
    settings: Arc<Settings>,
    generator: NameGenerator,
    release_date_regex: Regex,
}

impl DiskRenamer {
    pub fn new(settings: Arc<Settings>) -> Self {
        let generator = NameGenerator::new(settings.clone());
        let release_date_regex = Regex::new(r"\s+\(\d{4}(-\d{2}-\d{2})?\)$").unwrap();
        DiskRenamer { settings, generator, release_date_regex }
    }
}

#[async_trait]
impl Renamer for DiskRenamer {
    async fn find_options(&self, base_info: &BaseInfo, media_type: MediaFileType) -> eyre::Result<Option<RenamedMediaOptions>> {
        let media_path = Path::new(match media_type {
            MediaFileType::MOVIE => &self.settings.filesystem.movies_path,
            MediaFileType::TV => &self.settings.filesystem.tv_path,
            MediaFileType::UNKNOWN => {
                return Err(eyre!("unknown media type provided for base info {:?}", base_info));
            },
        });

        let mut name_variants = files::walk_dirs(media_path, self.settings.rename.max_depth)?
            .into_iter()
            .filter(|d| media_path != d.path())
            .map(|d| DiskPath::new(d, base_info.name(), &self.release_date_regex))
            .filter(|d| exclude_unsimilar(d, self.settings.rename.similarity_percent, base_info.name()))
            .collect::<Vec<DiskPath>>();

        // TODO: is this sorting highest similarity first?
        name_variants.sort_by(|a, b| a.similarity.cmp(&b.similarity));

        let name_variants: Vec<String> = name_variants.into_iter()
            .map(|d| d.file_name)
            .collect();

        if name_variants.is_empty() {
            return Ok(None);
        }

        Ok(Some(RenamedMediaOptions::new(MediaRenameOrigin::DISK, self.generator.generate_media_descriptions(name_variants))))
    }
}

// TODO: is this calculating ok? maybe in tests don't log to file, but to console
fn exclude_unsimilar(disk_path: &DiskPath, similarity_percent: u8, name: &str) -> bool {
    let bigger = std::cmp::max(disk_path.trimmed_file_name.len(), name.len());
    match u8::try_from((bigger - disk_path.similarity) / bigger * 100) {
        Ok(calculated_similarity) => {
            if calculated_similarity >= similarity_percent {
                info!("for path {}, the disk path {} is {}% similar with distance of {}", 
                    name, &disk_path.trimmed_file_name, calculated_similarity, disk_path.similarity);
                return true;
            }
            false
        },
        Err(e) => {
            warn!("could not convert to u8 {:?}", e);
            false
        },
    }
}