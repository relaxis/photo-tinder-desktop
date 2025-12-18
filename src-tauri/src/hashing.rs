//! Perceptual image hashing for similarity detection

use image::GenericImageView;
use std::path::Path;

const HASH_SIZE: u32 = 16; // 16x16 = 256 bits
pub const HAMMING_THRESHOLD: u32 = 10;

/// Compute dHash (difference hash) for an image
/// Returns a 64-character hex string (256 bits)
pub fn compute_dhash(image_path: &Path) -> Option<String> {
    // Load and resize image
    let img = match image::open(image_path) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Warning: Could not open image {}: {}", image_path.display(), e);
            return None;
        }
    };

    // Convert to grayscale and resize to (HASH_SIZE+1) x HASH_SIZE
    // We need one extra column to compute horizontal differences
    let gray = img.grayscale();
    let resized = image::imageops::resize(
        &gray.to_luma8(),
        HASH_SIZE + 1,
        HASH_SIZE,
        image::imageops::FilterType::Lanczos3,
    );

    // Compute difference hash
    // For each row, compare adjacent pixels: 1 if left > right, 0 otherwise
    let mut hash_bits = Vec::with_capacity((HASH_SIZE * HASH_SIZE) as usize);

    for y in 0..HASH_SIZE {
        for x in 0..HASH_SIZE {
            let left = resized.get_pixel(x, y)[0];
            let right = resized.get_pixel(x + 1, y)[0];
            hash_bits.push(left > right);
        }
    }

    // Convert bits to hex string
    let mut hex = String::with_capacity(64);
    for chunk in hash_bits.chunks(4) {
        let nibble = chunk.iter().enumerate().fold(0u8, |acc, (i, &bit)| {
            acc | ((bit as u8) << (3 - i))
        });
        hex.push_str(&format!("{:x}", nibble));
    }

    Some(hex)
}

/// Compute hamming distance between two hex hash strings
pub fn hamming_distance(hash1: &str, hash2: &str) -> u32 {
    if hash1.len() != hash2.len() {
        return u32::MAX;
    }

    let bytes1 = match hex_to_bytes(hash1) {
        Some(b) => b,
        None => return u32::MAX,
    };

    let bytes2 = match hex_to_bytes(hash2) {
        Some(b) => b,
        None => return u32::MAX,
    };

    bytes1.iter()
        .zip(bytes2.iter())
        .map(|(a, b)| (a ^ b).count_ones())
        .sum()
}

/// Convert hex string to bytes
fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i+2], 16).ok())
        .collect()
}

/// Cluster photos by perceptual hash similarity
/// Returns (clusters, photo_to_cluster mapping)
pub fn cluster_photos(
    photo_hashes: &std::collections::HashMap<String, String>,
) -> (std::collections::HashMap<String, Vec<String>>, std::collections::HashMap<String, String>) {
    use std::collections::HashMap;

    let mut clusters: HashMap<String, Vec<String>> = HashMap::new();
    let mut photo_to_cluster: HashMap<String, String> = HashMap::new();
    let mut cluster_reps: Vec<(String, String)> = Vec::new(); // (cluster_id, representative_hash)

    let mut cluster_count = 0;

    for (photo_id, hash) in photo_hashes {
        if hash.len() != 64 {
            continue;
        }

        let mut assigned = false;

        // Check against existing cluster representatives
        for (cluster_id, rep_hash) in &cluster_reps {
            let distance = hamming_distance(hash, rep_hash);
            if distance <= HAMMING_THRESHOLD {
                // Add to existing cluster
                clusters.get_mut(cluster_id).unwrap().push(photo_id.clone());
                photo_to_cluster.insert(photo_id.clone(), cluster_id.clone());
                assigned = true;
                break;
            }
        }

        if !assigned {
            // Create new cluster
            let cluster_id = format!("cluster_{:04}", cluster_count);
            clusters.insert(cluster_id.clone(), vec![photo_id.clone()]);
            cluster_reps.push((cluster_id.clone(), hash.clone()));
            photo_to_cluster.insert(photo_id.clone(), cluster_id);
            cluster_count += 1;
        }
    }

    (clusters, photo_to_cluster)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hamming_distance() {
        assert_eq!(hamming_distance("ff", "ff"), 0);
        assert_eq!(hamming_distance("ff", "00"), 8);
        assert_eq!(hamming_distance("f0", "0f"), 8);
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("ff00"), Some(vec![255, 0]));
        assert_eq!(hex_to_bytes("abc"), None); // Odd length
    }
}
