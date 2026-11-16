use std::path::{Path, PathBuf};

/// Get a storage reference out of a save path in this format:
/// `sha256/filename.extension`
pub fn decompose_save_path(path: impl Into<PathBuf>) -> String {
    let path = path.into();

    match (path.parent().and_then(Path::file_name), path.file_name()) {
        (Some(hash), Some(filename)) => {
            format!("{}/{}", hash.to_string_lossy(), filename.to_string_lossy())
        }
        _ => String::new(),
    }
}
