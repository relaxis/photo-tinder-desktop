//! Configuration management - handles user settings and persistence

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub source_folders: Vec<String>,
    pub accepted_folder: String,
    pub rejected_folder: String,
}

impl Config {
    /// Get the config directory path (OS-specific)
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("photo-tinder")
    }

    /// Get the config file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    /// Get the state file path
    pub fn state_path() -> PathBuf {
        Self::config_dir().join("state.json")
    }

    /// Get the hashes file path
    pub fn hashes_path() -> PathBuf {
        Self::config_dir().join("photo_hashes.json")
    }

    /// Load config from file, or return default
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Check if config is valid (has required folders set)
    pub fn is_valid(&self) -> bool {
        !self.source_folders.is_empty()
            && !self.accepted_folder.is_empty()
            && !self.rejected_folder.is_empty()
    }
}

/// Quick access locations for folder browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAccessLocation {
    pub name: String,
    pub path: String,
}

impl QuickAccessLocation {
    /// Get default quick access locations
    pub fn defaults() -> Vec<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let home_str = home.to_string_lossy().to_string();

        vec![
            Self {
                name: "Home".to_string(),
                path: home_str.clone(),
            },
            Self {
                name: "Pictures".to_string(),
                path: home.join("Pictures").to_string_lossy().to_string(),
            },
            Self {
                name: "Documents".to_string(),
                path: home.join("Documents").to_string_lossy().to_string(),
            },
            Self {
                name: "Downloads".to_string(),
                path: home.join("Downloads").to_string_lossy().to_string(),
            },
        ]
    }
}
