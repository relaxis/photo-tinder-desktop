//! Image management - scanning, moving, and undo operations

use crate::state::{ImageRecord, SUPPORTED_EXTENSIONS};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Generate a unique ID for an image based on its path
pub fn generate_image_id(path: &Path) -> String {
    let hash = md5::compute(path.to_string_lossy().as_bytes());
    format!("{:x}", hash)[..12].to_string()
}

/// Scan all source folders and return interleaved image records
pub fn scan_source_folders(source_folders: &[String]) -> Vec<ImageRecord> {
    let mut folder_images: Vec<Vec<ImageRecord>> = vec![Vec::new(); source_folders.len()];

    for (idx, folder_path) in source_folders.iter().enumerate() {
        let folder = Path::new(folder_path);
        if !folder.exists() {
            eprintln!("Warning: Source folder does not exist: {}", folder_path);
            continue;
        }

        // Use recursive scan for all folders
        let walker = WalkDir::new(folder).follow_links(true);

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Check extension
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if SUPPORTED_EXTENSIONS.contains(&ext_lower.as_str()) {
                    if let Ok(rel_path) = path.strip_prefix(folder) {
                        let img_id = generate_image_id(path);
                        folder_images[idx].push(ImageRecord {
                            id: img_id,
                            source_folder: folder_path.clone(),
                            relative_path: rel_path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    }

    // Interleave images from all folders (round-robin)
    let mut interleaved = Vec::new();
    let max_len = folder_images.iter().map(|v| v.len()).max().unwrap_or(0);

    for i in 0..max_len {
        for folder_imgs in &folder_images {
            if i < folder_imgs.len() {
                interleaved.push(folder_imgs[i].clone());
            }
        }
    }

    interleaved
}

/// Get destination path, handling filename collisions
pub fn get_destination_path(filename: &str, destination: &Path) -> PathBuf {
    let mut dest_path = destination.join(filename);

    if !dest_path.exists() {
        return dest_path;
    }

    // Handle collision by appending counter
    let stem = dest_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let extension = dest_path.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

    let mut counter = 1;
    while dest_path.exists() {
        let new_name = if extension.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, extension)
        };
        dest_path = destination.join(new_name);
        counter += 1;
    }

    dest_path
}

/// Move image to appropriate destination. Returns new path or None if skip.
pub fn move_image(
    record: &ImageRecord,
    decision: &str,
    accepted_folder: &str,
    rejected_folder: &str,
) -> Result<Option<String>, String> {
    if decision == "skipped" {
        return Ok(None);
    }

    let destination = if decision == "accepted" {
        Path::new(accepted_folder)
    } else {
        Path::new(rejected_folder)
    };

    // Ensure destination exists
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;

    let source_path = record.full_path();

    if !source_path.exists() {
        return Err(format!("Image not found: {}", source_path.display()));
    }

    let dest_path = get_destination_path(&record.filename(), destination);

    // Move file - try rename first, fall back to copy+delete for cross-filesystem
    if let Err(rename_err) = fs::rename(&source_path, &dest_path) {
        // Try copy + delete if rename fails (cross-filesystem)
        fs::copy(&source_path, &dest_path).map_err(|copy_err| {
            format!("Failed to move file: {} (rename: {}, copy: {})",
                source_path.display(), rename_err, copy_err)
        })?;
        fs::remove_file(&source_path).map_err(|del_err| {
            format!("File copied but failed to remove original: {}", del_err)
        })?;
    }

    Ok(Some(dest_path.to_string_lossy().to_string()))
}

/// Move file back to original location (undo)
pub fn undo_move(moved_path: &str, original_path: &str) -> Result<(), String> {
    let moved = Path::new(moved_path);
    let original = Path::new(original_path);

    if !moved.exists() {
        return Err(format!("Moved file not found: {}", moved_path));
    }

    // Ensure parent directory exists
    if let Some(parent) = original.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Move back - try rename first, fall back to copy+delete for cross-filesystem
    if let Err(rename_err) = fs::rename(moved, original) {
        // Try copy + delete if rename fails
        fs::copy(moved, original).map_err(|copy_err| {
            format!("Failed to restore file: {} (rename: {}, copy: {})",
                moved_path, rename_err, copy_err)
        })?;
        fs::remove_file(moved).map_err(|del_err| {
            format!("File restored but failed to remove from destination: {}", del_err)
        })?;
    }

    Ok(())
}

/// Build list of indices for images not yet decided
pub fn build_pending_indices(
    image_records: &[ImageRecord],
    decisions: &std::collections::HashMap<String, String>,
) -> Vec<usize> {
    let mut pending = Vec::new();

    for (i, record) in image_records.iter().enumerate() {
        let decision = decisions.get(&record.id);
        // Include if pending (no decision yet) OR skipped (recycle back into queue)
        if decision.is_none()
            || decision == Some(&"pending".to_string())
            || decision == Some(&"skipped".to_string())
        {
            // Check if file still exists
            if record.full_path().exists() {
                pending.push(i);
            }
        }
    }

    pending
}

/// Get the current image record to display
pub fn get_current_record<'a>(
    image_records: &'a [ImageRecord],
    pending_indices: &[usize],
    current_index: usize,
) -> Option<&'a ImageRecord> {
    if pending_indices.is_empty() {
        return None;
    }

    let idx = if current_index >= pending_indices.len() {
        0
    } else {
        current_index
    };

    pending_indices.get(idx).and_then(|&i| image_records.get(i))
}

/// Scan accepted folder for ranking mode
pub fn scan_accepted_photos(accepted_folder: &str) -> std::collections::HashMap<String, PathBuf> {
    let mut photos = std::collections::HashMap::new();
    let folder = Path::new(accepted_folder);

    if !folder.exists() {
        return photos;
    }

    for entry in WalkDir::new(folder).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if SUPPORTED_EXTENSIONS.contains(&ext_lower.as_str()) {
                let photo_id = generate_image_id(path);
                photos.insert(photo_id, path.to_path_buf());
            }
        }
    }

    photos
}

/// Browse a directory and return its contents
pub fn browse_directory(path: &str) -> Result<BrowseResult, String> {
    let dir_path = Path::new(path);

    if !dir_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    if !dir_path.is_dir() {
        return Err(format!("Not a directory: {}", path));
    }

    let parent = dir_path.parent().map(|p| p.to_string_lossy().to_string());

    let mut items = Vec::new();

    let entries = fs::read_dir(dir_path).map_err(|e| e.to_string())?;

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if name.starts_with('.') {
            continue;
        }

        let path = entry.path();
        let is_dir = path.is_dir();

        items.push(BrowseItem {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir,
        });
    }

    // Sort: directories first, then by name
    items.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(BrowseResult {
        current_path: path.to_string(),
        parent,
        items,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BrowseResult {
    pub current_path: String,
    pub parent: Option<String>,
    pub items: Vec<BrowseItem>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BrowseItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}
