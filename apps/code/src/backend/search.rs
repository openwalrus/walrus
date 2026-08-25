//! Ranking for a store that keeps no inverted index.
//!
//! BM25 needs a corpus, and one with no postings on disk gets it by
//! reading what it holds. The formula and the tokenizer are both
//! `store::text`'s, so a query ranks the same here as against a store
//! that keeps postings — only where the counts come from differs.

use store::text::{self, Bm25};

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
    let avgdl = tokenized.iter().map(|doc| doc.len()).sum::<usize>() as f64 / docs.len() as f64;
    if avgdl == 0.0 {
        return Vec::new();
    }

    let bm25 = Bm25::default();
    let mut scores = vec![0.0f64; docs.len()];
    for term in &terms {
        let counts: Vec<usize> = tokenized
            .iter()
            .map(|doc| doc.iter().filter(|t| *t == term).count())
            .collect();
        let df = counts.iter().filter(|count| **count > 0).count();
        if df == 0 {
            continue;
        }
        let idf = bm25.idf(df, docs.len());
        for (at, tf) in counts.iter().enumerate() {
            if *tf == 0 {
                continue;
            }
            let len = tokenized[at].len() as u32;
            scores[at] += idf * bm25.weigh(*tf as u32, len, avgdl) * docs[at].weight;
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
