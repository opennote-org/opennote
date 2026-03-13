use anyhow::{Context, Result};
use async_trait::async_trait;
use hf_hub::api::sync::ApiBuilder;
use serde_json::Value;
use std::{fs, path::PathBuf};

use crate::{downloader::Downloader, local_model::LocalModel};

pub struct HFDownloader;

#[async_trait]
impl Downloader for HFDownloader {
    async fn download_model(model: &str, use_mirror: bool) -> Result<LocalModel> {
        log::info!("===> Start downloading model files...");

        let mut api_builder: ApiBuilder = ApiBuilder::new().with_progress(true);

        if use_mirror {
            api_builder = api_builder.with_endpoint("https://hf-mirror.com".to_string());
        }

        let api = api_builder
            .build()
            .context("Failed to initialize HuggingFace API")?;

        let repo = api.model(model.to_string());

        // Download only required files
        let essential_files = [
            "model.safetensors",
            "config.json",
            "tokenizer.json",
            "tokenizer_config.json",
            "special_tokens_map.json",
        ];

        let mut model_root: Option<PathBuf> = None;
        let mut model_dim: Option<usize> = None;

        // Fetch repo metadata so we can verify file presence
        let info = repo.info().context("Failed to fetch repo info")?;

        // Download each required file if it exists in the repo
        for filename in essential_files {
            // Check whether the file exists in this repo snapshot
            if info.siblings.iter().any(|s| s.rfilename == filename) {
                match repo.get(filename) {
                    Ok(path) => {
                        log::debug!("Downloaded {} -> {:?}", filename, path.file_name());

                        // get model path
                        if model_root.is_none() {
                            model_root = path.parent().map(|p| p.to_path_buf());
                        }

                        // get model dimension
                        if filename == "config.json" {
                            let config_content =
                                fs::read_to_string(&path).context("Failed to get config.json")?;

                            match extract_hidden_size(&config_content) {
                                Some(dim) => model_dim = Some(dim),
                                None => log::warn!("Dimension not found in config.json"),
                            }
                        }
                    }
                    Err(e) => log::warn!("Failed to download {}: {}", filename, e),
                }
            } else {
                log::debug!("Skipping {}, not provided by model", filename);
            }
        }

        log::info!("===> Model files download completed...");

        let model_root = model_root.context("Failed to download any model files")?;
        let model_dim = model_dim.context("Model dimension not found in config.json")?;

        Ok(LocalModel {
            model_path: model_root,
            model_dim: model_dim,
        })
    }
}

/// 从 config.json 内容中提取 hidden_size
fn extract_hidden_size(config_content: &str) -> Option<usize> {
    let config: Value = serde_json::from_str(config_content).ok()?;

    // 尝试多个常见字段名
    let fields = [
        "hidden_size",
        "dim",
        "embedding_size",
        "d_model",
        "vector_dim",
    ];

    for field in fields {
        if let Some(value) = config.get(field) {
            if let Some(dim) = value.as_u64() {
                return Some(dim as usize);
            }
        }
    }

    None
}
