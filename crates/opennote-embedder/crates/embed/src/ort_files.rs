use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use sha2::{Digest, Sha256};

pub(crate) struct OnnxGraph {
    pub(crate) model: PathBuf,
    pub(crate) external_data: Option<PathBuf>,
}

struct SourceFile<'a> {
    name: &'a OsStr,
    resolved: PathBuf,
    len: u64,
}

impl<'a> SourceFile<'a> {
    fn resolve(path: &'a Path) -> Result<Self> {
        let name = path
            .file_name()
            .ok_or_else(|| anyhow!("Missing ONNX filename: {}", path.display()))?;
        let resolved = fs::canonicalize(path)?;
        let metadata = fs::metadata(&resolved)?;
        if !metadata.is_file() {
            return Err(anyhow!("Expected an ONNX file at {}", path.display()));
        }
        Ok(Self {
            name,
            resolved,
            len: metadata.len(),
        })
    }
}

/// Materialize the model set together if ONNX Runtime cannot use the original layout.
pub(crate) fn stage_onnx_graphs<const N: usize>(graphs: [OnnxGraph; N]) -> Result<[PathBuf; N]> {
    if !graphs.iter().any(|graph| graph.external_data.is_some()) {
        return Ok(graphs.map(|graph| graph.model));
    }

    let mut files = Vec::with_capacity(N * 2);
    let mut needs_staging = false;
    for graph in &graphs {
        let model = SourceFile::resolve(&graph.model)?;
        if let Some(path) = &graph.external_data {
            let data = SourceFile::resolve(path)?;
            // The Hub snapshot may point to a different blob directory for each file.
            needs_staging |= model.resolved.parent() != data.resolved.parent()
                || data.resolved.file_name() != Some(data.name);
            files.push(data);
        }
        files.push(model);
    }

    if !needs_staging {
        return Ok(graphs.map(|graph| graph.model));
    }

    let root = ProjectDirs::from("com", "manyougz", "OpenNote")
        .ok_or_else(|| anyhow!("Could not find the OpenNote cache directory"))?
        .cache_dir()
        .join("onnx-models");

    // The resolved Hub paths identify immutable blobs; lengths also catch truncated files.
    // Use the original basenames because the graph refers to its companion by filename.
    let mut key = Sha256::new();
    for file in &files {
        for part in [file.name, file.resolved.as_os_str()] {
            let bytes = part.as_encoded_bytes();
            key.update((bytes.len() as u64).to_le_bytes());
            key.update(bytes);
        }
        key.update(file.len.to_le_bytes());
    }

    let destination = root.join(format!("{:x}", key.finalize()));
    publish_model_set(&root, &destination, &files)
        .with_context(|| format!("Failed to stage ONNX models in {}", destination.display()))?;

    Ok(graphs.map(|graph| destination.join(graph.model.file_name().unwrap())))
}

fn publish_model_set(root: &Path, destination: &Path, files: &[SourceFile<'_>]) -> Result<()> {
    if is_complete(destination, files)? {
        return Ok(());
    }

    fs::create_dir_all(root)?;
    let staging = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(root)?;
    for file in files {
        let copied = fs::copy(&file.resolved, staging.path().join(file.name))?;
        if copied != file.len {
            return Err(anyhow!(
                "Source file changed while copying {}",
                file.resolved.display()
            ));
        }
    }

    // Publish only complete directories. If another process won the race, reuse its files.
    match fs::rename(staging.path(), destination) {
        Ok(()) => Ok(()),
        Err(_) if is_complete(destination, files)? => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn is_complete(path: &Path, files: &[SourceFile<'_>]) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            return Err(anyhow!(
                "Expected an ONNX cache directory at {}",
                path.display()
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    }

    for file in files {
        let path = path.join(file.name);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() && metadata.len() == file.len => {}
            Ok(_) => return Err(anyhow!("Invalid staged ONNX file: {}", path.display())),
            Err(error) => return Err(error.into()),
        }
    }

    Ok(true)
}
