//! Application state management

use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

/// Supported image extensions
/// Includes common formats, RAW formats from major camera manufacturers, and modern formats
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    // Common formats
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif",
    // Modern formats
    "heic", "heif", "avif", "jxl",
    // RAW formats
    "raw",
    "cr2", "cr3", "crw",        // Canon
    "nef", "nrw",               // Nikon
    "arw", "srf", "sr2",        // Sony
    "orf",                      // Olympus
    "rw2",                      // Panasonic
    "raf",                      // Fujifilm
    "pef", "ptx",               // Pentax
    "srw",                      // Samsung
    "x3f",                      // Sigma
    "dng",                      // Adobe DNG (universal RAW)
    "3fr", "fff",               // Hasselblad
    "iiq",                      // Phase One
    "rwl",                      // Leica
    "dcr", "kdc",               // Kodak
    "erf",                      // Epson
    "mrw",                      // Minolta
    "bay",                      // Casio
    "ari",                      // Arri
];

/// Represents a single image to be triaged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRecord {
    pub id: String,
    pub source_folder: String,
    pub relative_path: String,
}

impl ImageRecord {
    pub fn full_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.source_folder).join(&self.relative_path)
    }

    pub fn filename(&self) -> String {
        std::path::Path::new(&self.relative_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    pub fn source_name(&self) -> String {
        std::path::Path::new(&self.source_folder)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}

/// Persistent state that gets saved to disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentState {
    pub current_index: usize,
    pub decisions: HashMap<String, String>, // image_id -> "accepted"|"rejected"|"skipped"
    pub history: Vec<(String, String, String)>, // (image_id, old_decision, new_decision)
    pub moved_files: HashMap<String, String>, // image_id -> destination_path
    pub original_paths: HashMap<String, String>, // image_id -> original_path (for undo)
    pub mode: String, // "triage" or "ranking"
    pub ranking: RankingState,
}

impl PersistentState {
    /// Load state from file
    pub fn load() -> Self {
        let path = Config::state_path();
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(state) = serde_json::from_str(&contents) {
                    return state;
                }
            }
        }
        Self {
            mode: "triage".to_string(),
            ..Default::default()
        }
    }

    /// Save state to file
    pub fn save(&self) -> Result<(), String> {
        let path = Config::state_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())?;

        Ok(())
    }
}

/// Ranking mode state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RankingState {
    pub initialized: bool,
    pub ratings: HashMap<String, PhotoRating>,
    pub clusters: HashMap<String, Cluster>,
    pub photo_to_cluster: HashMap<String, String>,
    pub comparison_history: Vec<ComparisonRecord>,
    pub total_comparisons: usize,
    pub phase: String, // "intra_cluster" or "global"
    pub photo_count: usize,
    pub cluster_count: usize,
}

/// Rating for a single photo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoRating {
    pub mu: f64,
    pub sigma: f64,
    pub matches_played: usize,
}

impl Default for PhotoRating {
    fn default() -> Self {
        Self {
            mu: 1500.0,
            sigma: 350.0,
            matches_played: 0,
        }
    }
}

/// Cluster of similar photos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: String,
    pub photo_ids: Vec<String>,
    pub representative_id: Option<String>,
    pub internal_ranking_complete: bool,
}

/// Record of a comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonRecord {
    pub left_id: String,
    pub right_id: String,
    pub result: String,
    pub left_mu_before: f64,
    pub left_sigma_before: f64,
    pub right_mu_before: f64,
    pub right_sigma_before: f64,
    pub timestamp: f64,
}

/// Full application state (in-memory)
pub struct AppState {
    pub config: Mutex<Config>,
    pub persistent: Mutex<PersistentState>,
    pub image_records: Mutex<Vec<ImageRecord>>,
    pub pending_indices: Mutex<Vec<usize>>,
    pub photo_hashes: Mutex<HashMap<String, String>>,
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::load();
        let persistent = PersistentState::load();
        let photo_hashes = load_photo_hashes();

        Self {
            config: Mutex::new(config),
            persistent: Mutex::new(persistent),
            image_records: Mutex::new(Vec::new()),
            pending_indices: Mutex::new(Vec::new()),
            photo_hashes: Mutex::new(photo_hashes),
        }
    }
}

/// Load cached photo hashes from file
pub fn load_photo_hashes() -> HashMap<String, String> {
    let path = Config::hashes_path();
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(hashes) = serde_json::from_str(&contents) {
                return hashes;
            }
        }
    }
    HashMap::new()
}

/// Save photo hashes to file
pub fn save_photo_hashes(hashes: &HashMap<String, String>) -> Result<(), String> {
    let path = Config::hashes_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string_pretty(hashes).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(())
}
