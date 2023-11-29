use std::{
    cmp,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{PathBuf, Path},
};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use rand::{Rng, RngCore};
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
    async fn retrieve_all_by_base_and_type(
        &self,
        _base_info: &BaseInfo,
        _media_type: MediaFileType,
    ) -> eyre::Result<Vec<OnlineCacheItem>> {
        Ok(vec![])
    }
    async fn save_items(&self, _items: Vec<OnlineCacheItem>) -> eyre::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DownloadCacheRepo for EmptyDb {
    async fn retrieve_all_by_date_range(
        &self,
        _date_from: NaiveDateTime,
        _date_to: NaiveDateTime,
    ) -> eyre::Result<Vec<DownloadedMedia>> {
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
    let random_number = rng.gen::<u32>();
    let base_path = "/tmp/videosmover";
    settings.filesystem.downloads_path = format!("{}/{}/downloads", &base_path, random_number);
    settings.filesystem.movies_path = format!("{}/{}/movies", &base_path, random_number);
    settings.filesystem.tv_path = format!("{}/{}/tv", &base_path, random_number);

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

pub fn create_file(path: PathBuf, size: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();

    let f = File::create(&path).unwrap();
    let mut writer = BufWriter::new(f);

    let mut rng = rand::thread_rng();
    let mut buffer = [0; 1024];
    let mut remaining_size = size;

    while remaining_size > 0 {
        let to_write = cmp::min(remaining_size, buffer.len());
        let buffer = &mut buffer[..to_write];
        rng.fill_bytes(buffer);
        writer.write(buffer).unwrap();

        remaining_size -= to_write;
    }
}
