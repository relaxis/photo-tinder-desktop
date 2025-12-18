//! Glicko rating system for photo ranking

use crate::state::{PhotoRating, RankingState, Cluster};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::f64::consts::PI;

// Glicko constants
const GLICKO_Q: f64 = 0.0057565; // ln(10) / 400
const DEFAULT_MU: f64 = 1500.0;
const DEFAULT_SIGMA: f64 = 350.0;
const MIN_SIGMA: f64 = 50.0;

/// Glicko g-function: reduces impact based on opponent uncertainty
fn glicko_g(sigma: f64) -> f64 {
    1.0 / (1.0 + 3.0 * GLICKO_Q.powi(2) * sigma.powi(2) / PI.powi(2)).sqrt()
}

/// Expected score for player A vs player B
fn glicko_expected_score(mu_a: f64, mu_b: f64, sigma_b: f64) -> f64 {
    1.0 / (1.0 + 10_f64.powf(-glicko_g(sigma_b) * (mu_a - mu_b) / 400.0))
}

/// Update both ratings after a comparison
/// Returns ((new_winner_mu, new_winner_sigma), (new_loser_mu, new_loser_sigma))
pub fn glicko_update(
    winner_mu: f64, winner_sigma: f64,
    loser_mu: f64, loser_sigma: f64,
    is_tie: bool,
) -> ((f64, f64), (f64, f64)) {
    // Actual scores
    let (s_winner, s_loser) = if is_tie { (0.5, 0.5) } else { (1.0, 0.0) };

    // Expected scores
    let e_winner = glicko_expected_score(winner_mu, loser_mu, loser_sigma);
    let e_loser = glicko_expected_score(loser_mu, winner_mu, winner_sigma);

    // d-squared (rating change magnitude)
    let d_squared = |sigma_opp: f64, e: f64| -> f64 {
        let g_val = glicko_g(sigma_opp);
        1.0 / (GLICKO_Q.powi(2) * g_val.powi(2) * e * (1.0 - e) + 1e-10)
    };

    let d2_winner = d_squared(loser_sigma, e_winner);
    let d2_loser = d_squared(winner_sigma, e_loser);

    // New sigmas (uncertainty decreases with each match)
    let new_sigma_winner = (1.0 / (1.0 / winner_sigma.powi(2) + 1.0 / d2_winner)).sqrt();
    let new_sigma_loser = (1.0 / (1.0 / loser_sigma.powi(2) + 1.0 / d2_loser)).sqrt();

    // New mus
    let new_mu_winner = winner_mu + GLICKO_Q * new_sigma_winner.powi(2) * glicko_g(loser_sigma) * (s_winner - e_winner);
    let new_mu_loser = loser_mu + GLICKO_Q * new_sigma_loser.powi(2) * glicko_g(winner_sigma) * (s_loser - e_loser);

    // Apply floor to sigma
    let new_sigma_winner = new_sigma_winner.max(MIN_SIGMA);
    let new_sigma_loser = new_sigma_loser.max(MIN_SIGMA);

    ((new_mu_winner, new_sigma_winner), (new_mu_loser, new_sigma_loser))
}

/// Get conservative score (lower bound estimate): mu - 2*sigma
pub fn get_conservative_score(mu: f64, sigma: f64) -> f64 {
    mu - 2.0 * sigma
}

/// Select optimal pair for next comparison
pub fn select_pair(ranking: &RankingState) -> Option<(String, String)> {
    let ratings = &ranking.ratings;
    if ratings.len() < 2 {
        return None;
    }

    let phase = &ranking.phase;

    // Try intra-cluster pairing first
    if phase == "intra_cluster" && !ranking.clusters.is_empty() {
        if let Some(pair) = select_intra_cluster_pair(&ranking.clusters, ratings) {
            return Some(pair);
        }
        // All clusters done - caller should switch to global
    }

    // Global pairing
    select_global_pair(ratings)
}

/// Select a pair from within an incomplete cluster
fn select_intra_cluster_pair(
    clusters: &HashMap<String, Cluster>,
    ratings: &HashMap<String, PhotoRating>,
) -> Option<(String, String)> {
    for cluster in clusters.values() {
        if cluster.internal_ranking_complete {
            continue;
        }

        // Filter to photos that still exist in ratings
        let valid_ids: Vec<_> = cluster.photo_ids.iter()
            .filter(|pid| ratings.contains_key(*pid))
            .cloned()
            .collect();

        if valid_ids.len() < 2 {
            continue;
        }

        // Check if cluster is converged
        let avg_sigma: f64 = valid_ids.iter()
            .map(|pid| ratings.get(pid).map(|r| r.sigma).unwrap_or(DEFAULT_SIGMA))
            .sum::<f64>() / valid_ids.len() as f64;

        let min_matches = valid_ids.iter()
            .map(|pid| ratings.get(pid).map(|r| r.matches_played).unwrap_or(0))
            .min()
            .unwrap_or(0);

        // For small clusters, fewer matches needed
        let n_photos = valid_ids.len();
        let required_matches = if n_photos == 2 { 1 } else if n_photos == 3 { 2 } else { 3 };

        if avg_sigma < 100.0 || min_matches >= required_matches {
            continue; // Cluster is converged
        }

        // Select pair: highest sigma vs similar mu
        let mut sorted_by_sigma: Vec<_> = valid_ids.iter()
            .map(|pid| (pid.clone(), ratings.get(pid).map(|r| r.sigma).unwrap_or(DEFAULT_SIGMA)))
            .collect();
        sorted_by_sigma.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let primary = &sorted_by_sigma[0].0;
        let primary_mu = ratings.get(primary).map(|r| r.mu).unwrap_or(DEFAULT_MU);

        let candidates: Vec<String> = valid_ids.iter().filter(|p| *p != primary).cloned().collect();
        let opponent = candidates.iter()
            .min_by(|a, b| {
                let mu_a = ratings.get(a.as_str()).map(|r| r.mu).unwrap_or(DEFAULT_MU);
                let mu_b = ratings.get(b.as_str()).map(|r| r.mu).unwrap_or(DEFAULT_MU);
                (mu_a - primary_mu).abs().partial_cmp(&(mu_b - primary_mu).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(opp) = opponent {
            return Some((primary.clone(), opp.clone()));
        }
    }

    None
}

/// Select pair for global ranking phase
fn select_global_pair(ratings: &HashMap<String, PhotoRating>) -> Option<(String, String)> {
    let all_photos: Vec<_> = ratings.keys().cloned().collect();
    if all_photos.len() < 2 {
        return None;
    }

    // Sort by sigma descending
    let mut sorted_photos: Vec<_> = all_photos.iter()
        .map(|pid| (pid.clone(), ratings.get(pid).map(|r| r.sigma).unwrap_or(DEFAULT_SIGMA)))
        .collect();
    sorted_photos.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top N high-sigma candidates with some randomness
    let top_n = (10).max(sorted_photos.len() / 10);
    let primary_candidates: Vec<_> = sorted_photos.iter().take(top_n).map(|(p, _)| p.clone()).collect();

    let mut rng = rand::thread_rng();
    let primary = primary_candidates.choose(&mut rng)?;
    let primary_mu = ratings.get(primary).map(|r| r.mu).unwrap_or(DEFAULT_MU);

    // Find similar-mu opponent from a random sample
    let candidates: Vec<_> = all_photos.iter().filter(|p| *p != primary).cloned().collect();
    let sample_size = 20.min(candidates.len());

    let sampled: Vec<_> = if candidates.len() > sample_size {
        candidates.choose_multiple(&mut rng, sample_size).cloned().collect()
    } else {
        candidates.clone()
    };

    let opponent = sampled.iter()
        .min_by(|a, b| {
            let mu_a = ratings.get(a.as_str()).map(|r| r.mu).unwrap_or(DEFAULT_MU);
            let mu_b = ratings.get(b.as_str()).map(|r| r.mu).unwrap_or(DEFAULT_MU);
            (mu_a - primary_mu).abs().partial_cmp(&(mu_b - primary_mu).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

    Some((primary.clone(), opponent.clone()))
}

/// Initialize ratings for a set of photos
pub fn initialize_ratings(photo_ids: &[String]) -> HashMap<String, PhotoRating> {
    photo_ids.iter()
        .map(|id| (id.clone(), PhotoRating::default()))
        .collect()
}

/// Check if all intra-cluster comparisons are complete
pub fn check_intra_cluster_complete(clusters: &HashMap<String, Cluster>) -> bool {
    clusters.values().all(|c| c.internal_ranking_complete)
}

/// Mark a cluster as complete and set its representative
pub fn finalize_cluster(
    cluster: &mut Cluster,
    ratings: &HashMap<String, PhotoRating>,
) {
    cluster.internal_ranking_complete = true;

    // Set representative as highest-rated photo
    let best_id = cluster.photo_ids.iter()
        .filter(|pid| ratings.contains_key(*pid))
        .max_by(|a, b| {
            let score_a = ratings.get(*a).map(|r| get_conservative_score(r.mu, r.sigma)).unwrap_or(0.0);
            let score_b = ratings.get(*b).map(|r| get_conservative_score(r.mu, r.sigma)).unwrap_or(0.0);
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        });

    cluster.representative_id = best_id.cloned();
}
