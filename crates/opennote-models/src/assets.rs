use std::path::{Path, PathBuf};

/// Whether a destination is a managed `<sha256>/<filename>` asset reference.
pub fn is_asset_reference(reference: &str) -> bool {
    let (hash, filename) = match reference.split_once('/') {
        Some(parts) => parts,
        None => return false,
    };

    hash.len() == 64
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !filename.is_empty()
        && filename != "."
        && filename != ".."
        && !filename.contains(['/', '\\'])
}

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
