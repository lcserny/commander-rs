use std::{env, fs::OpenOptions};

use config::{Config, File, Environment};
use eyre::{Result, Context};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MongoDbConfig {
    pub connection_url: String,
    pub database: String,
    pub download_collection: String,
    pub online_collection: String,
}

#[derive(Debug, Deserialize)]
pub struct TmdbConfig {
    pub api_key: String,
    pub base_url: String,
    pub search_movies_url: String,
    pub movie_credits_url: String,
    pub search_tv_url: String,
    pub tv_credits_url: String,
}

#[derive(Debug, Deserialize)]
pub struct OnlineConfig {
    pub result_limit: u16,
    pub poster_base: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub command_name: String,
    pub command_listen_cron: String,
}

#[derive(Debug, Deserialize)]
pub struct FilesystemConfig {
    pub downloads_path: String,
    pub movies_path: String,
    pub tv_path: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchConfig {
    pub max_depth: u8,
    pub exclude_paths: Vec<String>,
    pub video_min_size_bytes: u64,
    pub video_mime_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenameConfig {
    pub max_depth: u8,
    pub trim_regex: Vec<String>,
    pub similarity_percent: u8,
}

#[derive(Debug, Deserialize)]
pub struct MoveConfig {
    pub subs_max_depth: u8,
    pub restricted_remove_paths: Vec<String>,
    pub subs_ext: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub mongodb: MongoDbConfig,
    pub tmdb: TmdbConfig,
    pub online: OnlineConfig,
    pub server: ServerConfig,
    pub filesystem: FilesystemConfig,
    pub search: SearchConfig,
    pub rename: RenameConfig,
    pub mv: MoveConfig,
    pub server_port: u16,
}

pub fn init_logging(log_file_path: &str) -> eyre::Result<()> { 
    // FIXME: this does not open already existing fiile correctly
    let file_appender = OpenOptions::new().create(true).write(true).open(log_file_path)?;
    tracing_subscriber::fmt().with_writer(file_appender).init(); 
    Ok(())
} 
 
pub fn init_config(filename: &str, env_prefix: &str) -> Result<Settings> { 
    let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into()); 
 
    Config::builder() 
                .add_source(File::with_name(filename)) 
                .add_source(File::with_name(&format!("{}_{}", filename, run_mode)).required(false)) 
                .add_source(Environment::with_prefix(env_prefix)) 
                .build()? 
                .try_deserialize().wrap_err_with(|| format!("failed to create Settings from config proovided: {}", &filename))
} 
