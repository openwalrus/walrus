//! Utility functions for memory recall scoring and ranking.

use agent::MemoryEntry;
use std::collections::HashSet;

/// Jaccard similarity between two strings (tokenized by whitespace).
pub(crate) fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a.split_whitespace().collect();
    let set_b: HashSet<&str> = b.split_whitespace().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

/// Maximal Marginal Relevance re-ranking.
///
/// Selects items that balance relevance (score) against diversity
/// (dissimilarity to already-selected items). Lambda controls the
/// trade-off: 1.0 = pure relevance, 0.0 = pure diversity.
pub(crate) fn mmr_rerank(
    candidates: Vec<(MemoryEntry, f64)>,
    limit: usize,
    mmr_lambda: f64,
) -> Vec<MemoryEntry> {
    let mut remaining: Vec<(MemoryEntry, f64)> = candidates;
    let mut selected: Vec<MemoryEntry> = Vec::with_capacity(limit);

    while selected.len() < limit && !remaining.is_empty() {
        let mut best_idx = 0;
        let mut best_mmr = f64::NEG_INFINITY;

        for (i, (entry, score)) in remaining.iter().enumerate() {
            let max_sim = selected
                .iter()
                .map(|s| jaccard_similarity(&entry.value, &s.value))
                .fold(0.0_f64, f64::max);
            let mmr_score = mmr_lambda * score - (1.0 - mmr_lambda) * max_sim;
            if mmr_score > best_mmr {
                best_mmr = mmr_score;
                best_idx = i;
            }
        }

        let (entry, _) = remaining.remove(best_idx);
        selected.push(entry);
    }

    selected
}

/// Return the current unix timestamp in seconds.
pub(crate) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
}
