use std::{io::Read, path::{Path, PathBuf}};

use anyhow::Result;
use log::info;
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::AsyncWriteExt;

pub trait LoadAndSave 
where 
    Self: Serialize + DeserializeOwned
{
    fn new(path: &str) -> Self;
    
    fn get_path(&self) -> &Path;
    
    fn load(path: &str) -> Result<Self> {
        let mut json: String = String::new();
        match std::fs::File::open(path) {
            Ok(mut result) => {
                let _ = result.read_to_string(&mut json);
                Ok(serde_json::from_str(&json)?)
            }
            Err(_) => {
                let path_buf = PathBuf::from(path);
                
                if !path_buf.parent().unwrap().exists() {
                    match std::fs::create_dir_all(path_buf.parent().unwrap()) {
                        Ok(_) => info!("Data directory `{}` created", path_buf.parent().unwrap().display()),
                        Err(error) => return Err(error.into()),
                    }
                } else {
                    info!("Data directory `{}` exists, skip creation", path_buf.parent().unwrap().display())
                }
                
                Ok(Self::new(path))
            },
        }
    }

    async fn save(&self) -> Result<()> {
        let mut file: tokio::fs::File = tokio::fs::File::create(self.get_path()).await?;
        let buffer: String = serde_json::to_string_pretty(self)?;
        let _ = file.write(buffer.as_bytes()).await;

        Ok(())
    }
}
