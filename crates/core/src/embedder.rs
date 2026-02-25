//! Embedding trait for converting text to vector representations.
//!
//! Used by memory backends that support semantic search (e.g. walrus-sqlite).

use std::future::Future;

/// Converts text into a dense vector embedding.
///
/// Implementations may call external APIs (OpenAI, local models, etc.).
/// Uses RPITIT for async without boxing.
pub trait Embedder: Send + Sync {
    /// Embed the given text into a dense float vector.
    fn embed(&self, text: &str) -> impl Future<Output = Vec<f32>> + Send;
}