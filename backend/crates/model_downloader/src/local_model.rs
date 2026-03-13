use std::{fmt, path::PathBuf};

#[derive(Debug, Clone)]
pub struct LocalModel {
    pub model_path: PathBuf,
    pub model_dim: usize,
}

impl fmt::Display for LocalModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LocalModel {{ model_path: {}, model_dim: {} }}",
            self.model_path.to_string_lossy(),
            self.model_dim,
        )
    }
}
