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