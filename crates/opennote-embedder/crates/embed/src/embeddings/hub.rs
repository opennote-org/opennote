use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use hf_hub::{HFClientBuilder, HFClientSync, HFError, HFRepositorySync, RepoTypeModel};

pub const HUGGINGFACE_CHINESE_MIRROR: &str = "https://hf-mirror.com";

/// Test the connectivity between the computer and huggingface
pub fn is_huggingface_connected() -> bool {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            let client = reqwest::Client::new();

            match client.get("https://huggingface.co").send().await {
                Ok(_) => true,
                Err(_) => false,
            }
        })
    })
}

/// Blocking Hugging Face Hub client shared by model repositories created during one load.
#[derive(Clone)]
pub(crate) struct HubClient {
    inner: HFClientSync,
}

impl HubClient {
    pub(crate) fn new(token: Option<&str>) -> Result<Self> {
        let endpoint = match is_huggingface_connected() {
            true => None,
            false => Some(HUGGINGFACE_CHINESE_MIRROR),
        };

        let mut builder = HFClientBuilder::new();

        if let Some(endpoint) = endpoint {
            builder = builder.endpoint(endpoint);
        }

        if let Some(token) = token {
            builder = builder.token(token);
        }

        Ok(Self {
            inner: builder.build_sync()?,
        })
    }

    pub(crate) fn model(&self, model_id: &str, revision: Option<&str>) -> HubModelRepo {
        let (owner, name) = hf_hub::split_id(model_id);
        HubModelRepo {
            inner: self.inner.model(owner, name),
            revision: revision.map(str::to_owned),
        }
    }
}

/// A model repository with its revision attached to every file download.
#[derive(Clone)]
pub(crate) struct HubModelRepo {
    inner: HFRepositorySync<RepoTypeModel>,
    revision: Option<String>,
}

impl HubModelRepo {
    pub(crate) fn new(model_id: &str, revision: Option<&str>, token: Option<&str>) -> Result<Self> {
        Ok(HubClient::new(token)?.model(model_id, revision))
    }

    pub(crate) fn get(&self, filename: &str) -> hf_hub::HFResult<PathBuf> {
        self.inner
            .download_file()
            .filename(filename)
            .maybe_revision(self.revision.clone())
            .send()
    }

    pub(crate) fn optional(&self, filename: &str) -> hf_hub::HFResult<Option<PathBuf>> {
        match self.get(filename) {
            Ok(path) => Ok(Some(path)),
            Err(HFError::EntryNotFound { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Returns the first existing file, falling through only when a file is absent.
    pub(crate) fn first_available(&self, filenames: &[&str]) -> hf_hub::HFResult<PathBuf> {
        let mut last_missing = None;
        for filename in filenames {
            match self.get(filename) {
                Ok(path) => return Ok(path),
                Err(error @ HFError::EntryNotFound { .. }) => last_missing = Some(error),
                Err(error) => return Err(error),
            }
        }

        Err(last_missing.unwrap_or_else(|| {
            HFError::InvalidParameter("at least one Hub filename is required".to_string())
        }))
    }

    pub(crate) fn safetensor_shards(&self, index_filename: &str) -> Result<Vec<PathBuf>> {
        let index_path = self
            .get(index_filename)
            .with_context(|| format!("failed to download safetensors index {index_filename}"))?;
        let index_file = std::fs::File::open(&index_path).with_context(|| {
            format!("failed to open safetensors index {}", index_path.display())
        })?;
        let index: serde_json::Value = serde_json::from_reader(index_file).with_context(|| {
            format!("failed to parse safetensors index {}", index_path.display())
        })?;
        let weight_map = index
            .get("weight_map")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| anyhow!("weight_map is missing from {}", index_path.display()))?;

        let mut filenames: Vec<&str> = weight_map
            .values()
            .filter_map(serde_json::Value::as_str)
            .collect();
        filenames.sort_unstable();
        filenames.dedup();

        filenames
            .into_iter()
            .map(|filename| {
                self.get(filename)
                    .with_context(|| format!("failed to download safetensors shard {filename}"))
            })
            .collect()
    }
}
