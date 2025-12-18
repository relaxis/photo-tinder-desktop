//! Photo Tinder Desktop - Image Triage and Ranking Application

pub mod commands;
pub mod config;
pub mod hashing;
pub mod image_manager;
pub mod ranking;
pub mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Config
            commands::get_config,
            commands::save_config,
            commands::is_config_valid,
            // Triage
            commands::initialize_app,
            commands::get_current_image,
            commands::swipe,
            commands::undo,
            commands::get_preload_list,
            // Mode
            commands::get_mode,
            commands::set_mode,
            // Ranking
            commands::get_ranking_stats,
            commands::init_ranking,
            commands::get_pair,
            commands::compare,
            commands::undo_ranking,
            commands::get_leaderboard,
            // Folders
            commands::get_folders,
            commands::add_source_folder,
            commands::remove_source_folder,
            commands::set_destination_folder,
            commands::browse,
            commands::get_home_dir,
            // Photo browser
            commands::get_photos_by_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
