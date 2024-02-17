use std::sync::Arc;

use crate::{download::DownloadCacheRepo, rename::cache::OnlineCacheRepo};

// Add more repos as needed

pub trait DbWrapper: DownloadCacheRepo + OnlineCacheRepo 
{
    fn download_cache_repo(&self) -> &dyn DownloadCacheRepo;
    fn online_cache_repo(&self) -> &dyn OnlineCacheRepo;
}

impl <R> DbWrapper for R 
    where R: DownloadCacheRepo + OnlineCacheRepo 
{
    fn download_cache_repo(&self) -> &dyn DownloadCacheRepo {
        self
    }

    fn online_cache_repo(&self) -> &dyn OnlineCacheRepo {
        self
    }
}

#[derive(Clone)]
pub struct DbClient {
    db: Arc<dyn DbWrapper>,
}

impl DbClient {
    pub fn new(db: Arc<dyn DbWrapper>) -> Self {
        DbClient { db } 
    }

    pub fn download_cache_repo(&self) -> &dyn DownloadCacheRepo {
        self.db.download_cache_repo()
    }

    pub fn online_cache_repo(&self) -> &dyn OnlineCacheRepo {
        self.db.online_cache_repo()
    }
}