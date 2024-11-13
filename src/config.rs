//! The definition of a configuration for the harmonizer
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Defines a configuration. It is Ser/De-able with serde.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub merger_path: PathBuf,
    pub sync_path: PathBuf,
    pub min_run: i32,
    pub max_run: i32,
}

impl Config {
    /// Load a configuration from a YAML file.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(eyre!(
                "Attempted to load configuration from non-existant path: {}",
                path.display()
            ));
        }

        let yaml_str = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str::<Self>(&yaml_str)?)
    }

    /// Save this configuration to a YAML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let yaml_str = serde_yaml::to_string(self)?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(yaml_str.as_bytes())?;
        Ok(())
    }
}
