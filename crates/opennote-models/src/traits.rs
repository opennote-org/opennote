use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Migrate the old configuration file structure to the new one
pub trait MigrateConfigurationFileStructure
where
    Self: LoadFromAndSaveToFile,
{
    fn migrate(self, configuration_folder_path: &PathBuf) -> Result<Self> {
        // load the default
        let default = serde_json::to_value(Self::default())?;
        let mut current = serde_json::to_value(&self)?;

        // find the missing fields
        // insert the missing fields with default values
        Self::recurse(&default, &mut current);

        // save the config
        let migrated: Self = serde_json::from_value(current)?;
        migrated.save_to_file(configuration_folder_path)?;

        Ok(migrated)
    }

    fn recurse(default: &Value, current: &mut Value) {
        // Missing -> Insert the default value
        // Not missing -> Recurse
        if default.is_object() {
            for (key, value) in default.as_object().unwrap() {
                match current.get_mut(key) {
                    Some(next) => Self::recurse(value, next),
                    None => {
                        if let Some(current) = current.as_object_mut() {
                            current.insert(key.to_owned(), value.to_owned());
                        }
                    }
                }
            }
        }
    }
}

pub trait LoadFromAndSaveToFile
where
    Self: Default + Sized + Serialize + DeserializeOwned,
{
    /// Implement this to get the load and save automatically implemented.
    fn get_configuration_filename() -> &'static str;

    /// The path to the configuration file's directory.
    /// It will automatically add the configuration file name
    fn load_from_file(configuration_folder_path: impl AsRef<Path>) -> Result<Self> {
        let path = configuration_folder_path.as_ref();

        let path = if path.is_dir() {
            path.join(Self::get_configuration_filename())
        } else {
            path.to_path_buf()
        };

        let default_settings = Self::default();
        if !path.exists() {
            write_to_file(&default_settings, &path)?;
            return Ok(default_settings);
        }

        let content: String = match std::fs::read_to_string(&path) {
            Ok(result) => result,
            Err(_error) => return Ok(default_settings),
        };

        serde_json::from_str(&content)
            .context(format!("Failed to parse config file: {}", path.display()))
    }

    fn save_to_file(&self, configuration_folder_path: &PathBuf) -> Result<()> {
        write_to_file(
            &self,
            &configuration_folder_path.join(Self::get_configuration_filename()),
        )?;
        Ok(())
    }
}

fn write_to_file<T: Sized + Serialize>(content: &T, path: &PathBuf) -> Result<()> {
    let content = serde_json::to_string_pretty(content).context("Failed to serialize config")?;

    std::fs::write(path, content)
        .with_context(|| format!("Failed to write file: {}", path.display()))?;

    Ok(())
}
