//! Tauri commands - Functions callable from JavaScript

use crate::config::{Config, QuickAccessLocation};
use crate::hashing::{compute_dhash, cluster_photos};
use crate::image_manager::{
    browse_directory, build_pending_indices, get_current_record, move_image,
    scan_accepted_photos, scan_source_folders, undo_move,
};
use crate::ranking::{glicko_update, select_pair, get_conservative_score, initialize_ratings};
use crate::state::{AppState, Cluster, ComparisonRecord, save_photo_hashes};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

// ============================================================================
// Response types
// ============================================================================

#[derive(Serialize)]
pub struct ImageInfo {
    pub done: bool,
    pub id: Option<String>,
    pub index: usize,
    pub total_pending: usize,
    pub total_images: usize,
    pub filename: Option<String>,
    pub source_folder: Option<String>,
    pub file_path: Option<String>,
    pub stats: Stats,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct Stats {
    pub total: usize,
    pub pending: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub skipped: usize,
    pub processed: usize,
}

#[derive(Serialize)]
pub struct SwipeResult {
    pub success: bool,
    pub decision: String,
}

#[derive(Serialize)]
pub struct UndoResult {
    pub success: bool,
    pub message: String,
    pub image_id: Option<String>,
}

#[derive(Serialize)]
pub struct PairInfo {
    pub done: bool,
    pub error: bool,
    pub message: Option<String>,
    pub left: Option<PhotoInfo>,
    pub right: Option<PhotoInfo>,
    pub stats: Option<RankingStats>,
}

#[derive(Serialize)]
pub struct PhotoInfo {
    pub id: String,
    pub mu: f64,
    pub sigma: f64,
    pub matches: usize,
    pub file_path: String,
}

#[derive(Serialize)]
pub struct RankingStats {
    pub initialized: bool,
    pub total_photos: usize,
    pub total_comparisons: usize,
    pub cluster_count: usize,
    pub phase: String,
    pub high_uncertainty: usize,
    pub medium_uncertainty: usize,
    pub low_uncertainty: usize,
    pub avg_matches_per_photo: f64,
}

#[derive(Serialize)]
pub struct LeaderboardPhoto {
    pub id: String,
    pub mu: f64,
    pub sigma: f64,
    pub matches: usize,
    pub score: f64,
    pub file_path: String,
}

#[derive(Serialize)]
pub struct FolderInfo {
    pub path: String,
    pub exists: bool,
    pub photo_count: usize,
    pub decided_count: usize,
}

#[derive(Serialize)]
pub struct FoldersResponse {
    pub folders: Vec<FolderInfo>,
    pub accepted_folder: String,
    pub rejected_folder: String,
}

#[derive(Serialize)]
pub struct BrowseResponse {
    pub error: bool,
    pub message: Option<String>,
    pub current_path: Option<String>,
    pub parent: Option<String>,
    pub items: Vec<crate::image_manager::BrowseItem>,
    pub quick_access: Vec<QuickAccessLocation>,
}

// ============================================================================
// Configuration commands
// ============================================================================

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(config: Config, state: State<AppState>) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    *cfg = config.clone();
    cfg.save()?;

    // Rescan images with new config
    let records = scan_source_folders(&cfg.source_folders);
    let mut image_records = state.image_records.lock().unwrap();
    *image_records = records;

    let persistent = state.persistent.lock().unwrap();
    let pending = build_pending_indices(&image_records, &persistent.decisions);
    let mut pending_indices = state.pending_indices.lock().unwrap();
    *pending_indices = pending;

    Ok(())
}

#[tauri::command]
pub fn is_config_valid(state: State<AppState>) -> bool {
    state.config.lock().unwrap().is_valid()
}

// ============================================================================
// Triage mode commands
// ============================================================================

#[tauri::command]
pub fn initialize_app(state: State<AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap();

    if !config.is_valid() {
        return Ok(()); // Config not set up yet
    }

    // Scan source folders
    let records = scan_source_folders(&config.source_folders);
    let mut image_records = state.image_records.lock().unwrap();
    *image_records = records;

    // Build pending indices
    let persistent = state.persistent.lock().unwrap();
    let pending = build_pending_indices(&image_records, &persistent.decisions);
    let mut pending_indices = state.pending_indices.lock().unwrap();
    *pending_indices = pending;

    Ok(())
}

#[tauri::command]
pub fn get_current_image(state: State<AppState>) -> ImageInfo {
    let _config = state.config.lock().unwrap();
    let persistent = state.persistent.lock().unwrap();
    let image_records = state.image_records.lock().unwrap();
    let pending_indices = state.pending_indices.lock().unwrap();

    let stats = get_stats_data(&image_records, &persistent.decisions);

    let record = get_current_record(&image_records, &pending_indices, persistent.current_index);

    match record {
        Some(r) => ImageInfo {
            done: false,
            id: Some(r.id.clone()),
            index: persistent.current_index,
            total_pending: pending_indices.len(),
            total_images: image_records.len(),
            filename: Some(r.filename()),
            source_folder: Some(r.source_name()),
            file_path: Some(r.full_path().to_string_lossy().to_string()),
            stats,
            message: None,
        },
        None => ImageInfo {
            done: true,
            id: None,
            index: 0,
            total_pending: 0,
            total_images: image_records.len(),
            filename: None,
            source_folder: None,
            file_path: None,
            stats,
            message: Some("All images have been triaged!".to_string()),
        },
    }
}

fn get_stats_data(
    image_records: &[crate::state::ImageRecord],
    decisions: &HashMap<String, String>,
) -> Stats {
    let accepted = decisions.values().filter(|d| *d == "accepted").count();
    let rejected = decisions.values().filter(|d| *d == "rejected").count();
    let skipped = decisions.values().filter(|d| *d == "skipped").count();

    let processed = accepted + rejected + skipped;
    Stats {
        total: image_records.len(),
        pending: image_records.len().saturating_sub(processed),
        accepted,
        rejected,
        skipped,
        processed,
    }
}

#[tauri::command]
pub fn swipe(image_id: String, direction: String, state: State<AppState>) -> Result<SwipeResult, String> {
    let config = state.config.lock().unwrap();
    let mut persistent = state.persistent.lock().unwrap();
    let image_records = state.image_records.lock().unwrap();

    // Find the record
    let record = image_records.iter().find(|r| r.id == image_id)
        .ok_or("Image not found")?;

    // Map direction to decision
    let decision = match direction.as_str() {
        "left" => "rejected",
        "right" => "accepted",
        "down" => "skipped",
        _ => return Err("Invalid direction".to_string()),
    };

    // Record old decision for history
    let old_decision = persistent.decisions.get(&image_id).cloned().unwrap_or("pending".to_string());

    // Move file if accept/reject
    if let Some(new_path) = move_image(record, decision, &config.accepted_folder, &config.rejected_folder)? {
        persistent.original_paths.insert(image_id.clone(), record.full_path().to_string_lossy().to_string());
        persistent.moved_files.insert(image_id.clone(), new_path);
    }

    // Update state
    persistent.decisions.insert(image_id.clone(), decision.to_string());
    persistent.history.push((image_id, old_decision, decision.to_string()));

    // Trim history
    if persistent.history.len() > 100 {
        let keep = persistent.history.len() - 100;
        persistent.history = persistent.history.split_off(keep);
    }

    // Rebuild pending list
    let pending = build_pending_indices(&image_records, &persistent.decisions);
    let mut pending_indices = state.pending_indices.lock().unwrap();
    *pending_indices = pending;

    // Save state
    persistent.save()?;

    Ok(SwipeResult {
        success: true,
        decision: decision.to_string(),
    })
}

#[tauri::command]
pub fn undo(state: State<AppState>) -> Result<UndoResult, String> {
    let mut persistent = state.persistent.lock().unwrap();
    let image_records = state.image_records.lock().unwrap();

    if persistent.history.is_empty() {
        return Ok(UndoResult {
            success: false,
            message: "Nothing to undo".to_string(),
            image_id: None,
        });
    }

    // Pop last decision
    let (image_id, old_decision, new_decision) = persistent.history.pop().unwrap();

    // If file was moved, move it back
    if new_decision == "accepted" || new_decision == "rejected" {
        if let (Some(moved_path), Some(original_path)) = (
            persistent.moved_files.get(&image_id),
            persistent.original_paths.get(&image_id),
        ) {
            undo_move(moved_path, original_path)?;
            persistent.moved_files.remove(&image_id);
            persistent.original_paths.remove(&image_id);
        }
    }

    // Restore old decision
    if old_decision == "pending" {
        persistent.decisions.remove(&image_id);
    } else {
        persistent.decisions.insert(image_id.clone(), old_decision.clone());
    }

    // Rebuild pending
    let pending = build_pending_indices(&image_records, &persistent.decisions);
    let mut pending_indices = state.pending_indices.lock().unwrap();

    // Find the undone image in pending
    for (i, &idx) in pending.iter().enumerate() {
        if image_records[idx].id == image_id {
            persistent.current_index = i;
            break;
        }
    }

    *pending_indices = pending;
    persistent.save()?;

    Ok(UndoResult {
        success: true,
        message: format!("Undone: {} -> {}", new_decision, old_decision),
        image_id: Some(image_id),
    })
}

#[tauri::command]
pub fn get_preload_list(state: State<AppState>) -> Vec<String> {
    let persistent = state.persistent.lock().unwrap();
    let image_records = state.image_records.lock().unwrap();
    let pending_indices = state.pending_indices.lock().unwrap();

    let mut ids = Vec::new();
    for i in 1..=6 {
        let idx = persistent.current_index + i;
        if idx < pending_indices.len() {
            if let Some(record) = image_records.get(pending_indices[idx]) {
                ids.push(record.full_path().to_string_lossy().to_string());
            }
        }
    }
    ids
}

// ============================================================================
// Mode commands
// ============================================================================

#[tauri::command]
pub fn get_mode(state: State<AppState>) -> String {
    state.persistent.lock().unwrap().mode.clone()
}

#[tauri::command]
pub fn set_mode(mode: String, state: State<AppState>) -> Result<(), String> {
    if mode != "triage" && mode != "ranking" {
        return Err("Invalid mode".to_string());
    }

    let mut persistent = state.persistent.lock().unwrap();
    persistent.mode = mode;
    persistent.save()
}

// ============================================================================
// Ranking mode commands
// ============================================================================

#[tauri::command]
pub fn get_ranking_stats(state: State<AppState>) -> RankingStats {
    let persistent = state.persistent.lock().unwrap();
    let ranking = &persistent.ranking;

    if !ranking.initialized {
        return RankingStats {
            initialized: false,
            total_photos: 0,
            total_comparisons: 0,
            cluster_count: 0,
            phase: "not_initialized".to_string(),
            high_uncertainty: 0,
            medium_uncertainty: 0,
            low_uncertainty: 0,
            avg_matches_per_photo: 0.0,
        };
    }

    let ratings = &ranking.ratings;
    let total_photos = ratings.len();

    let high_uncertainty = ratings.values().filter(|r| r.sigma >= 200.0).count();
    let medium_uncertainty = ratings.values().filter(|r| r.sigma >= 100.0 && r.sigma < 200.0).count();
    let low_uncertainty = ratings.values().filter(|r| r.sigma < 100.0).count();

    let avg_matches = if total_photos > 0 {
        ratings.values().map(|r| r.matches_played).sum::<usize>() as f64 / total_photos as f64
    } else {
        0.0
    };

    RankingStats {
        initialized: true,
        total_photos,
        total_comparisons: ranking.total_comparisons,
        cluster_count: ranking.cluster_count,
        phase: ranking.phase.clone(),
        high_uncertainty,
        medium_uncertainty,
        low_uncertainty,
        avg_matches_per_photo: (avg_matches * 100.0).round() / 100.0,
    }
}

#[tauri::command]
pub fn init_ranking(state: State<AppState>) -> Result<RankingStats, String> {
    let config = state.config.lock().unwrap();
    let mut persistent = state.persistent.lock().unwrap();
    let mut photo_hashes = state.photo_hashes.lock().unwrap();

    // Scan accepted photos
    let photos = scan_accepted_photos(&config.accepted_folder);
    if photos.is_empty() {
        return Err("No photos found in Accepted folder".to_string());
    }

    // Initialize ratings
    let photo_ids: Vec<_> = photos.keys().cloned().collect();
    let ratings = initialize_ratings(&photo_ids);

    // Compute hashes for photos that don't have them
    for (photo_id, path) in &photos {
        if !photo_hashes.contains_key(photo_id) {
            if let Some(hash) = compute_dhash(path) {
                photo_hashes.insert(photo_id.clone(), hash);
            }
        }
    }

    // Save hashes
    save_photo_hashes(&photo_hashes)?;

    // Cluster photos
    let (clusters_raw, photo_to_cluster) = cluster_photos(&photo_hashes);

    // Convert to Cluster structs
    let clusters: HashMap<String, Cluster> = clusters_raw.into_iter()
        .map(|(id, photo_ids)| {
            let complete = photo_ids.len() < 2;
            (id.clone(), Cluster {
                id,
                photo_ids,
                representative_id: None,
                internal_ranking_complete: complete,
            })
        })
        .collect();

    // Update ranking state
    persistent.ranking.initialized = true;
    persistent.ranking.ratings = ratings;
    persistent.ranking.clusters = clusters.clone();
    persistent.ranking.photo_to_cluster = photo_to_cluster;
    persistent.ranking.comparison_history = Vec::new();
    persistent.ranking.total_comparisons = 0;
    persistent.ranking.phase = if clusters.is_empty() { "global".to_string() } else { "intra_cluster".to_string() };
    persistent.ranking.photo_count = photos.len();
    persistent.ranking.cluster_count = clusters.len();

    persistent.save()?;

    Ok(get_ranking_stats_internal(&persistent.ranking))
}

fn get_ranking_stats_internal(ranking: &crate::state::RankingState) -> RankingStats {
    let ratings = &ranking.ratings;
    let total_photos = ratings.len();

    let high_uncertainty = ratings.values().filter(|r| r.sigma >= 200.0).count();
    let medium_uncertainty = ratings.values().filter(|r| r.sigma >= 100.0 && r.sigma < 200.0).count();
    let low_uncertainty = ratings.values().filter(|r| r.sigma < 100.0).count();

    let avg_matches = if total_photos > 0 {
        ratings.values().map(|r| r.matches_played).sum::<usize>() as f64 / total_photos as f64
    } else {
        0.0
    };

    RankingStats {
        initialized: ranking.initialized,
        total_photos,
        total_comparisons: ranking.total_comparisons,
        cluster_count: ranking.cluster_count,
        phase: ranking.phase.clone(),
        high_uncertainty,
        medium_uncertainty,
        low_uncertainty,
        avg_matches_per_photo: (avg_matches * 100.0).round() / 100.0,
    }
}

#[tauri::command]
pub fn get_pair(state: State<AppState>) -> PairInfo {
    let config = state.config.lock().unwrap();
    let persistent = state.persistent.lock().unwrap();

    if !persistent.ranking.initialized {
        return PairInfo {
            done: false,
            error: true,
            message: Some("Ranking not initialized".to_string()),
            left: None,
            right: None,
            stats: None,
        };
    }

    let pair = select_pair(&persistent.ranking);

    match pair {
        Some((left_id, right_id)) => {
            let ratings = &persistent.ranking.ratings;

            let left_rating = ratings.get(&left_id).cloned().unwrap_or_default();
            let right_rating = ratings.get(&right_id).cloned().unwrap_or_default();

            // Get file paths from accepted folder
            let photos = scan_accepted_photos(&config.accepted_folder);
            let left_path = photos.get(&left_id).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            let right_path = photos.get(&right_id).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

            PairInfo {
                done: false,
                error: false,
                message: None,
                left: Some(PhotoInfo {
                    id: left_id,
                    mu: (left_rating.mu * 10.0).round() / 10.0,
                    sigma: (left_rating.sigma * 10.0).round() / 10.0,
                    matches: left_rating.matches_played,
                    file_path: left_path,
                }),
                right: Some(PhotoInfo {
                    id: right_id,
                    mu: (right_rating.mu * 10.0).round() / 10.0,
                    sigma: (right_rating.sigma * 10.0).round() / 10.0,
                    matches: right_rating.matches_played,
                    file_path: right_path,
                }),
                stats: Some(get_ranking_stats_internal(&persistent.ranking)),
            }
        }
        None => PairInfo {
            done: true,
            error: false,
            message: Some("No more pairs available".to_string()),
            left: None,
            right: None,
            stats: Some(get_ranking_stats_internal(&persistent.ranking)),
        },
    }
}

#[tauri::command]
pub fn compare(left_id: String, right_id: String, result: String, state: State<AppState>) -> Result<(), String> {
    let mut persistent = state.persistent.lock().unwrap();

    if !persistent.ranking.initialized {
        return Err("Ranking not initialized".to_string());
    }

    let ratings = &mut persistent.ranking.ratings;

    let left = ratings.get(&left_id).ok_or("Left photo not found")?.clone();
    let right = ratings.get(&right_id).ok_or("Right photo not found")?.clone();

    // Store for undo
    let record = ComparisonRecord {
        left_id: left_id.clone(),
        right_id: right_id.clone(),
        result: result.clone(),
        left_mu_before: left.mu,
        left_sigma_before: left.sigma,
        right_mu_before: right.mu,
        right_sigma_before: right.sigma,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64(),
    };

    if result != "skip" {
        let is_tie = result == "tie";

        let (winner_mu, winner_sigma, loser_mu, loser_sigma) = if result == "left" || is_tie {
            (left.mu, left.sigma, right.mu, right.sigma)
        } else {
            (right.mu, right.sigma, left.mu, left.sigma)
        };

        let ((new_winner_mu, new_winner_sigma), (new_loser_mu, new_loser_sigma)) =
            glicko_update(winner_mu, winner_sigma, loser_mu, loser_sigma, is_tie);

        // Apply updates
        if result == "left" || is_tie {
            ratings.get_mut(&left_id).unwrap().mu = new_winner_mu;
            ratings.get_mut(&left_id).unwrap().sigma = new_winner_sigma;
            ratings.get_mut(&right_id).unwrap().mu = new_loser_mu;
            ratings.get_mut(&right_id).unwrap().sigma = new_loser_sigma;
        } else {
            ratings.get_mut(&right_id).unwrap().mu = new_winner_mu;
            ratings.get_mut(&right_id).unwrap().sigma = new_winner_sigma;
            ratings.get_mut(&left_id).unwrap().mu = new_loser_mu;
            ratings.get_mut(&left_id).unwrap().sigma = new_loser_sigma;
        }

        // Increment match counts
        ratings.get_mut(&left_id).unwrap().matches_played += 1;
        ratings.get_mut(&right_id).unwrap().matches_played += 1;
    }

    // Record comparison
    persistent.ranking.comparison_history.push(record);
    persistent.ranking.total_comparisons += 1;

    // Trim history
    if persistent.ranking.comparison_history.len() > 100 {
        let keep = persistent.ranking.comparison_history.len() - 100;
        persistent.ranking.comparison_history = persistent.ranking.comparison_history.split_off(keep);
    }

    // Check if we should switch from intra_cluster to global
    if persistent.ranking.phase == "intra_cluster" {
        let all_complete = persistent.ranking.clusters.values().all(|c| c.internal_ranking_complete);
        if all_complete {
            persistent.ranking.phase = "global".to_string();
        }
    }

    persistent.save()
}

#[tauri::command]
pub fn undo_ranking(state: State<AppState>) -> Result<UndoResult, String> {
    let mut persistent = state.persistent.lock().unwrap();

    if persistent.ranking.comparison_history.is_empty() {
        return Ok(UndoResult {
            success: false,
            message: "Nothing to undo".to_string(),
            image_id: None,
        });
    }

    let record = persistent.ranking.comparison_history.pop().unwrap();
    let ratings = &mut persistent.ranking.ratings;

    // Restore ratings
    if let Some(left) = ratings.get_mut(&record.left_id) {
        left.mu = record.left_mu_before;
        left.sigma = record.left_sigma_before;
        if record.result != "skip" {
            left.matches_played = left.matches_played.saturating_sub(1);
        }
    }

    if let Some(right) = ratings.get_mut(&record.right_id) {
        right.mu = record.right_mu_before;
        right.sigma = record.right_sigma_before;
        if record.result != "skip" {
            right.matches_played = right.matches_played.saturating_sub(1);
        }
    }

    persistent.ranking.total_comparisons = persistent.ranking.total_comparisons.saturating_sub(1);
    persistent.save()?;

    Ok(UndoResult {
        success: true,
        message: format!("Undone comparison: {}", record.result),
        image_id: None,
    })
}

#[tauri::command]
pub fn get_leaderboard(limit: usize, state: State<AppState>) -> Vec<LeaderboardPhoto> {
    let config = state.config.lock().unwrap();
    let persistent = state.persistent.lock().unwrap();

    if !persistent.ranking.initialized {
        return Vec::new();
    }

    let ratings = &persistent.ranking.ratings;
    let photos = scan_accepted_photos(&config.accepted_folder);

    let mut scored: Vec<_> = ratings.iter()
        .map(|(id, rating)| {
            let score = get_conservative_score(rating.mu, rating.sigma);
            let file_path = photos.get(id).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            LeaderboardPhoto {
                id: id.clone(),
                mu: (rating.mu * 10.0).round() / 10.0,
                sigma: (rating.sigma * 10.0).round() / 10.0,
                matches: rating.matches_played,
                score: (score * 10.0).round() / 10.0,
                file_path,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    scored
}

// ============================================================================
// Folder management commands
// ============================================================================

#[tauri::command]
pub fn get_folders(state: State<AppState>) -> FoldersResponse {
    let config = state.config.lock().unwrap();
    let persistent = state.persistent.lock().unwrap();
    let image_records = state.image_records.lock().unwrap();

    let folders: Vec<FolderInfo> = config.source_folders.iter()
        .map(|folder_path| {
            let exists = std::path::Path::new(folder_path).exists();
            let photo_count = image_records.iter().filter(|r| r.source_folder == *folder_path).count();
            let decided_count = image_records.iter()
                .filter(|r| r.source_folder == *folder_path && persistent.decisions.contains_key(&r.id))
                .count();

            FolderInfo {
                path: folder_path.clone(),
                exists,
                photo_count,
                decided_count,
            }
        })
        .collect();

    FoldersResponse {
        folders,
        accepted_folder: config.accepted_folder.clone(),
        rejected_folder: config.rejected_folder.clone(),
    }
}

#[tauri::command]
pub fn add_source_folder(path: String, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();

    if !std::path::Path::new(&path).exists() {
        return Err(format!("Folder does not exist: {}", path));
    }

    if config.source_folders.contains(&path) {
        return Err("Folder already added".to_string());
    }

    config.source_folders.push(path);
    config.save()?;

    // Rescan
    let records = scan_source_folders(&config.source_folders);
    let mut image_records = state.image_records.lock().unwrap();
    *image_records = records;

    let persistent = state.persistent.lock().unwrap();
    let pending = build_pending_indices(&image_records, &persistent.decisions);
    let mut pending_indices = state.pending_indices.lock().unwrap();
    *pending_indices = pending;

    Ok(())
}

#[tauri::command]
pub fn remove_source_folder(path: String, clear_decisions: bool, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    let mut persistent = state.persistent.lock().unwrap();
    let image_records = state.image_records.lock().unwrap();

    if !config.source_folders.contains(&path) {
        return Err("Folder not found".to_string());
    }

    // Optionally clear decisions
    if clear_decisions {
        let to_remove: Vec<_> = image_records.iter()
            .filter(|r| r.source_folder == path)
            .map(|r| r.id.clone())
            .collect();

        for img_id in to_remove {
            persistent.decisions.remove(&img_id);
            persistent.moved_files.remove(&img_id);
            persistent.original_paths.remove(&img_id);
        }
    }

    config.source_folders.retain(|f| f != &path);
    config.save()?;
    persistent.save()?;

    // Rescan
    drop(image_records);
    let records = scan_source_folders(&config.source_folders);
    let mut image_records = state.image_records.lock().unwrap();
    *image_records = records;

    let pending = build_pending_indices(&image_records, &persistent.decisions);
    let mut pending_indices = state.pending_indices.lock().unwrap();
    *pending_indices = pending;

    Ok(())
}

#[tauri::command]
pub fn set_destination_folder(folder_type: String, path: String, state: State<AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();

    // Create folder if it doesn't exist
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;

    match folder_type.as_str() {
        "accepted" => config.accepted_folder = path,
        "rejected" => config.rejected_folder = path,
        _ => return Err("Invalid folder type".to_string()),
    }

    config.save()
}

#[tauri::command]
pub fn browse(path: String) -> BrowseResponse {
    match browse_directory(&path) {
        Ok(result) => BrowseResponse {
            error: false,
            message: None,
            current_path: Some(result.current_path),
            parent: result.parent,
            items: result.items,
            quick_access: QuickAccessLocation::defaults(),
        },
        Err(e) => BrowseResponse {
            error: true,
            message: Some(e),
            current_path: None,
            parent: None,
            items: Vec::new(),
            quick_access: QuickAccessLocation::defaults(),
        },
    }
}

#[tauri::command]
pub fn get_home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string())
}

// ============================================================================
// Photo browser commands
// ============================================================================

#[derive(Serialize)]
pub struct BrowsePhotosResponse {
    pub photos: Vec<BrowsePhotoInfo>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

#[derive(Serialize, Clone)]
pub struct BrowsePhotoInfo {
    pub id: String,
    pub filename: String,
    pub file_path: String,
    pub mu: Option<f64>,
    pub sigma: Option<f64>,
    pub score: Option<f64>,
    pub matches: Option<usize>,
}

#[tauri::command]
pub fn get_photos_by_status(
    status: String,
    sort: String,
    page: usize,
    per_page: usize,
    state: State<AppState>,
) -> BrowsePhotosResponse {
    let config = state.config.lock().unwrap();
    let persistent = state.persistent.lock().unwrap();

    // Determine which folder to scan
    let folder = if status == "accepted" {
        &config.accepted_folder
    } else {
        &config.rejected_folder
    };

    // Scan the folder for photos
    let photos_map = scan_accepted_photos(folder);

    // Get ranking data if available
    let rankings = if persistent.ranking.initialized {
        Some(&persistent.ranking.ratings)
    } else {
        None
    };

    // Build photo list with optional ranking info
    let mut photos: Vec<BrowsePhotoInfo> = photos_map
        .iter()
        .map(|(id, path)| {
            let filename = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let (mu, sigma, score, matches) = if let Some(ratings) = rankings {
                if let Some(rating) = ratings.get(id) {
                    let s = get_conservative_score(rating.mu, rating.sigma);
                    (
                        Some((rating.mu * 10.0).round() / 10.0),
                        Some((rating.sigma * 10.0).round() / 10.0),
                        Some((s * 10.0).round() / 10.0),
                        Some(rating.matches_played),
                    )
                } else {
                    (None, None, None, None)
                }
            } else {
                (None, None, None, None)
            };

            BrowsePhotoInfo {
                id: id.clone(),
                filename,
                file_path: path.to_string_lossy().to_string(),
                mu,
                sigma,
                score,
                matches,
            }
        })
        .collect();

    // Sort based on the requested sort order
    match sort.as_str() {
        "ranking" => {
            // Best first (highest score)
            photos.sort_by(|a, b| {
                let score_a = a.score.unwrap_or(0.0);
                let score_b = b.score.unwrap_or(0.0);
                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        "ranking_asc" => {
            // Worst first (lowest score)
            photos.sort_by(|a, b| {
                let score_a = a.score.unwrap_or(0.0);
                let score_b = b.score.unwrap_or(0.0);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        "recent" => {
            // Sort by file modification time (most recent first)
            photos.sort_by(|a, b| {
                let time_a = std::fs::metadata(&a.file_path)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let time_b = std::fs::metadata(&b.file_path)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                time_b.cmp(&time_a)
            });
        }
        "name" | _ => {
            // Sort by filename
            photos.sort_by(|a, b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
        }
    }

    let total = photos.len();
    let total_pages = if total == 0 { 1 } else { (total + per_page - 1) / per_page };

    // Paginate
    let start = page.saturating_sub(1) * per_page;
    let end = (start + per_page).min(total);
    let paginated = if start < total {
        photos[start..end].to_vec()
    } else {
        Vec::new()
    };

    BrowsePhotosResponse {
        photos: paginated,
        total,
        page,
        per_page,
        total_pages,
    }
}
