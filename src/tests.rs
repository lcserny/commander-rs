use std::{
    fs::{self, File},
    io::Write,
    path::{PathBuf, Path},
};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use rand::Rng;
use testcontainers::{core::WaitFor, GenericImage};

use crate::{
    config::{init_config, Settings},
    download::{DownloadCacheRepo, DownloadedMedia},
    rename::{
        name::BaseInfo,
        cache::{OnlineCacheItem, OnlineCacheRepo},
        MediaFileType,
    },
};

pub const MONGO_PORT: u16 = 27017;
pub const MONGO_USER: &str = "root";
pub const MONGO_PASS: &str = "rootpass";

pub struct EmptyDb;

#[async_trait]
impl OnlineCacheRepo for EmptyDb {
    async fn retrieve_all_by_base_and_type( &self, _base_info: &BaseInfo, _media_type: MediaFileType,) -> eyre::Result<Vec<OnlineCacheItem>> {
        Ok(vec![])
    }

    async fn save_items(&self, _items: Vec<OnlineCacheItem>) -> eyre::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DownloadCacheRepo for EmptyDb {
    async fn retrieve_all_by_date_range( &self, _date_from: NaiveDateTime, _date_to: NaiveDateTime,) -> eyre::Result<Vec<DownloadedMedia>> {
        Ok(vec![])
    }

    async fn persist(&self, _items: Vec<DownloadedMedia>) -> eyre::Result<()> {
        Ok(())
    }
}

fn init_test_logging() {
    let _r = tracing_subscriber::fmt().pretty().try_init();
}

pub fn create_test_settings() -> Settings {
    init_test_logging();
    let mut settings = init_config("config/settings_test", "TST_CMDR").unwrap();

    let mut rng = rand::thread_rng();
    let random_number = rng.gen::<u32>().to_string();
    
    let base_path = std::env::temp_dir().join("videosmover").join(&random_number);
    settings.filesystem.downloads_path = base_path.join("downloads").to_string_lossy().into_owned();
    settings.filesystem.movies_path = base_path.join("movies").to_string_lossy().into_owned();
    settings.filesystem.tv_path = base_path.join("tv").to_string_lossy().into_owned();

    fs::create_dir_all(Path::new(&settings.filesystem.downloads_path)).unwrap();
    fs::create_dir_all(Path::new(&settings.filesystem.movies_path)).unwrap();
    fs::create_dir_all(Path::new(&settings.filesystem.tv_path)).unwrap();

    settings
}

pub fn create_mongo_image() -> GenericImage {
    GenericImage::new("mongo", "5.0")
        .with_exposed_port(MONGO_PORT)
        .with_env_var("MONGO_INITDB_ROOT_USERNAME", MONGO_USER)
        .with_env_var("MONGO_INITDB_ROOT_PASSWORD", MONGO_PASS)
        .with_wait_for(WaitFor::message_on_stdout("Waiting for connections"))
}

// if size is more than 20, valid video file content will be filled to given path
pub fn create_file(path: PathBuf, size: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let mut f = File::create(&path).unwrap();
    let a = fs::read("tests/resources/video.mp4").unwrap();
    f.write_all(&a[..size]).unwrap();
}
