use std::{env, fs::OpenOptions};

use config::{Config, File, Environment};
use eyre::{Result, Context};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct MongoDb {
    pub connection_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub mongodb: MongoDb,
    pub server_port: u16,
}

pub fn init_logging(log_file_path: &str) -> eyre::Result<()> { 
    let file_appender = OpenOptions::new().create(true).write(true).open(log_file_path)?;
    tracing_subscriber::fmt().with_writer(file_appender).init(); 
    Ok(())
} 
 
pub fn init_config(filename: &str, env_prefix: &str) -> Result<Settings> { 
    let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into()); 
 
    return Config::builder() 
                .add_source(File::with_name(filename)) 
                .add_source(File::with_name(&format!("{}_{}", filename, run_mode)).required(false)) 
                .add_source(Environment::with_prefix(env_prefix)) 
                .build()? 
                .try_deserialize().wrap_err_with(|| format!("failed to create Settings from config proovided: {}", &filename)); 
} 
