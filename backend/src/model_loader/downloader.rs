use anyhow::{Context, Result};
use hf_hub::api::sync::ApiBuilder;
use std::{fmt, path::PathBuf};

#[derive(Debug, Clone)]
pub struct ModelLocalPaths {
    pub model_root_path: PathBuf,
}

impl fmt::Display for ModelLocalPaths {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ModelLocalPaths {{ model_root_path: {} }}",
            self.model_root_path.to_string_lossy(),
        )
    }
}

/// Download model (defaults to CN mirror)
pub async fn download_model(model_id: &str) -> Result<ModelLocalPaths> {
    download_model_with_config(model_id, true).await
}

pub async fn download_model_with_config(
    model_id: &str,
    use_mirror: bool,
) -> Result<ModelLocalPaths> {
    log::info!("===> Start downloading model files...");

    let mut api_builder: ApiBuilder = ApiBuilder::new().with_progress(true);

    if use_mirror {
        api_builder = api_builder.with_endpoint("https://hf-mirror.com".to_string());
    }

    let api = api_builder
        .build()
        .context("Failed to initialize HuggingFace API")?;

    let repo = api.model(model_id.to_string());

    // Download only required files
    let essential_files = [
        "model.safetensors",
        "config.json",
        "tokenizer.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ];

    let mut model_root: Option<PathBuf> = None;
    // Fetch repo metadata so we can verify file presence
    let info = repo.info().context("Failed to fetch repo info")?;

    // Download each required file if it exists in the repo
    for filename in essential_files {
        // Check whether the file exists in this repo snapshot
        if info.siblings.iter().any(|s| s.rfilename == filename) {
            match repo.get(filename) {
                Ok(path) => {
                    log::debug!("Downloaded {} -> {:?}", filename, path.file_name());
                    if model_root.is_none() {
                        model_root = path.parent().map(|p| p.to_path_buf());
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
    log::debug!("model_root: {}", model_root.to_string_lossy());

    Ok(ModelLocalPaths {
        model_root_path: model_root,
    })
}
