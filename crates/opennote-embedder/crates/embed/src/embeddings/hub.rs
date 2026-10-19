use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use hf_hub::{
    HFClientBuilder, HFClientSync, HFError, HFRepositorySync, HFResult, RepoTypeModel,
    repository::RepoTreeEntry,
};

pub const HUGGINGFACE_CHINESE_MIRROR: &str = "https://hf-mirror.com";

/// Blocking Hugging Face Hub clients shared during one model load.
#[derive(Clone)]
pub(crate) struct HubClient {
    inner: HFClientSync,
    mirror: HFClientSync,
}

impl HubClient {
    pub(crate) fn new(token: Option<&str>) -> Result<Self> {
        let mut primary = HFClientBuilder::new();
        let mut mirror = HFClientBuilder::new().endpoint(HUGGINGFACE_CHINESE_MIRROR);

        if let Some(token) = token {
            primary = primary.token(token);
            mirror = mirror.token(token);
        }

        // Constructing clients does not make network requests.
        Ok(Self {
            inner: primary.build_sync()?,
            mirror: mirror.build_sync()?,
        })
    }

    pub(crate) fn model(&self, model_id: &str, revision: Option<&str>) -> HubModelRepo {
        let (owner, name) = hf_hub::split_id(model_id);
        HubModelRepo {
            inner: self.inner.model(owner, name),
            mirror: self.mirror.model(owner, name),
            revision: revision.map(str::to_owned),
        }
    }
}

/// A model repository with its revision attached to every file download.
#[derive(Clone)]
pub(crate) struct HubModelRepo {
    inner: HFRepositorySync<RepoTypeModel>,
    mirror: HFRepositorySync<RepoTypeModel>,
    revision: Option<String>,
}

impl HubModelRepo {
    pub(crate) fn new(model_id: &str, revision: Option<&str>, token: Option<&str>) -> Result<Self> {
        Ok(HubClient::new(token)?.model(model_id, revision))
    }

    fn get_cached(&self, filename: &str) -> HFResult<PathBuf> {
        self.inner
            .download_file()
            .filename(filename)
            .maybe_revision(self.revision.clone())
            .local_files_only(true)
            .send()
    }

    fn with_mirror<T>(
        &self,
        request: impl Fn(&HFRepositorySync<RepoTypeModel>) -> HFResult<T>,
    ) -> HFResult<T> {
        match request(&self.inner) {
            Ok(value) => Ok(value),
            Err(error @ HFError::Request { .. }) => {
                tracing::warn!(
                    error = %error,
                    "Primary model request failed; trying the mirror"
                );
                request(&self.mirror)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn get(&self, filename: &str) -> HFResult<PathBuf> {
        match self.get_cached(filename) {
            Ok(path) => return Ok(path),
            Err(HFError::LocalEntryNotFound { .. }) => {}
            Err(error) => return Err(error),
        }

        self.with_mirror(|repo| {
            repo.download_file()
                .filename(filename)
                .maybe_revision(self.revision.clone())
                .send()
        })
    }

    /// Detect the format from cached weights before querying the Hub.
    ///
    /// These filenames match the default ONNX and native BERT loaders.
    /// Other layouts retain the existing remote detection behavior.
    pub(crate) fn is_onnx(&self) -> HFResult<bool> {
        for (filename, is_onnx) in [
            ("model.onnx", true),
            ("model.safetensors", false),
            ("pytorch_model.bin", false),
        ] {
            match self.get_cached(filename) {
                Ok(_) => return Ok(is_onnx),
                Err(HFError::LocalEntryNotFound { .. }) => {}
                Err(error) => return Err(error),
            }
        }

        Ok(self
            .inspect_files(false)?
            .iter()
            .any(|filename| filename.ends_with(".onnx")))
    }

    /// List files in this repo.
    pub(crate) fn inspect_files(&self, recursive: bool) -> HFResult<Vec<String>> {
        let entries = self.with_mirror(|repo| {
            repo.list_tree()
                .maybe_revision(self.revision.clone())
                .recursive(recursive)
                .send()
        })?;

        Ok(entries
            .into_iter()
            .filter_map(|entry| match entry {
                RepoTreeEntry::File { path, .. } => Some(path),
                _ => None,
            })
            .collect())
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
        for filename in filenames {
            match self.get_cached(filename) {
                Ok(path) => return Ok(path),
                Err(HFError::LocalEntryNotFound { .. }) => {}
                Err(error) => return Err(error),
            }
        }

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
