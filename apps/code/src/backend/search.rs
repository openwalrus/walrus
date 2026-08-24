//! Ranking for a store that keeps no inverted index.
//!
//! BM25 needs a corpus, and one with no postings on disk gets it by
//! reading what it holds. The formula and its constants are the ones in
//! `store::text`, and the tokenizer is literally that module's, so a
//! query ranks the same here as it does against the daemon.

use store::text;

/// BM25's two knobs, as `store::text` publishes them.
const K1: f64 = 1.2;
const B: f64 = 0.75;

/// One document to rank: its text, and what a match in it is worth.
pub struct Doc {
    pub text: String,
    pub weight: f64,
}

/// Indices into `docs` with their scores, best first, at most `limit`.
///
/// An empty or all-stopword query ranks nothing rather than everything —
/// a search box is allowed to contain junk.
pub fn rank(docs: &[Doc], query: &str, limit: usize) -> Vec<(usize, f64)> {
    let terms: Vec<String> = text::tokenize(query)
        .into_iter()
        .filter(|t| !text::is_stopword(t))
        .collect();
    if terms.is_empty() || limit == 0 || docs.is_empty() {
        return Vec::new();
    }

    let tokenized: Vec<Vec<String>> = docs.iter().map(|d| text::tokenize(&d.text)).collect();
    let n = docs.len() as f64;
    let avgdl = tokenized.iter().map(|t| t.len()).sum::<usize>() as f64 / n;
    if avgdl == 0.0 {
        return Vec::new();
    }

    let mut scores = vec![0.0f64; docs.len()];
    for term in &terms {
        let counts: Vec<usize> = tokenized
            .iter()
            .map(|doc| doc.iter().filter(|t| *t == term).count())
            .collect();
        let df = counts.iter().filter(|c| **c > 0).count() as f64;
        if df == 0.0 {
            continue;
        }
        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
        for (at, tf) in counts.iter().enumerate() {
            if *tf == 0 {
                continue;
            }
            let (tf, dl) = (*tf as f64, tokenized[at].len() as f64);
            let norm = (tf * (K1 + 1.0)) / (tf + K1 * (1.0 - B + B * dl / avgdl));
            scores[at] += idf * norm * docs[at].weight;
        }
    }

    let mut ranked: Vec<(usize, f64)> = scores
        .into_iter()
        .enumerate()
        .filter(|(_, score)| *score > 0.0)
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranked.truncate(limit);
    ranked
}
