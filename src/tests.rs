use std::{path::PathBuf, cmp, fs::{self, File}, io::{BufWriter, Write}};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use rand::RngCore;
use testcontainers::{GenericImage, core::WaitFor};

use crate::{rename::{online_cache::{OnlineCacheRepo, OnlineCacheItem}, name::BaseInfo, MediaFileType}, download::{DownloadCacheRepo, DownloadedMedia}, config::{Settings, init_config}};

pub const MONGO_PORT: u16 = 27017;
pub const MONGO_USER: &str = "root";
pub const MONGO_PASS: &str = "rootpass";

pub struct EmptyDb;

#[async_trait]
impl OnlineCacheRepo for EmptyDb {
    async fn retrieve_all_by_base_and_type(&self, _base_info: &BaseInfo, _media_type: MediaFileType) -> eyre::Result<Vec<OnlineCacheItem>> {
        Ok(vec![])
    }
    async fn save_items(&self, _items: Vec<OnlineCacheItem>) -> eyre::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DownloadCacheRepo for EmptyDb {
    async fn retrieve_all_by_date_range(&self, _date_from: NaiveDateTime, _date_to: NaiveDateTime) -> eyre::Result<Vec<DownloadedMedia>> {
        Ok(vec![])
    }

    async fn persist(&self, _items: Vec<DownloadedMedia>) -> eyre::Result<()> {
        Ok(())
    }
}

fn init_test_logging() { 
    tracing_subscriber::fmt().pretty().init(); 
} 

pub fn create_test_settings() -> Settings {
    init_test_logging();
    init_config("config/settings_test", "TST_CMDR").unwrap()
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