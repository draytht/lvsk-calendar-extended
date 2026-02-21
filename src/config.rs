use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

use crate::sync::google::GoogleConfig;

#[derive(Debug, Deserialize, Default)]
pub struct AppConfig {
    pub google: Option<GoogleConfig>,
    pub sync:   Option<SyncConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SyncConfig {
    pub interval_seconds: Option<u64>,
    pub auto_sync:        Option<bool>,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path = config_dir().join("config.toml");
        if path.exists() {
            Ok(toml::from_str(&std::fs::read_to_string(&path)?)?)
        } else {
            Ok(AppConfig::default())
        }
    }
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lifemanager")
}
