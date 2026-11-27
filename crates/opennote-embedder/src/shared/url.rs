use anyhow::Result;
use reqwest::Url;
use tokio::task::JoinHandle;

/// Parse an asset reference as a remote asset url.
///
/// Only `http` and `https` are accepted, since `Url::parse` also accepts
/// things like Windows paths (`C:\image.png` has the scheme `c`).
pub fn parse_remote_asset_url(reference: &str) -> Option<Url> {
    match Url::parse(reference) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => Some(url),
        _ => None,
    }
}

#[derive(Debug)]
pub struct UrlAssetsDownloader {
    task_handles: Vec<JoinHandle<Result<Vec<u8>, reqwest::Error>>>,
}

impl UrlAssetsDownloader {
    pub fn new() -> Self {
        Self {
            task_handles: Vec::new(),
        }
    }

    /// Dispatch an url asset download task
    pub async fn start_download_url_asset_task(&mut self, url: Url) {
        self.task_handles.push(tokio::spawn(async move {
            let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
            Ok::<Vec<u8>, reqwest::Error>(bytes.to_vec())
        }));
    }

    /// Any download task was scheduled?
    pub fn has_tasks(&self) -> bool {
        !self.task_handles.is_empty()
    }

    /// Consume this downloader, return all results of existing download tasks as in memory bytes
    pub async fn fetch_downloaded_tasks(self) -> Result<Vec<Vec<u8>>> {
        let mut image_bytes = Vec::new();
        for handle in self.task_handles.into_iter() {
            // Fail all to let users retry
            image_bytes.push(handle.await??);
        }

        Ok(image_bytes)
    }
}
