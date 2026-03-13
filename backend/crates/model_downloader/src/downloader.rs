use anyhow::Result;
use async_trait::async_trait;

use crate::local_model::LocalModel;

#[async_trait]
pub trait Downloader {
    async fn download_model(model: &str, use_mirror: bool) -> Result<LocalModel>;
}
