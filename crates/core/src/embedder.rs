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

#[cfg(test)]
mod tests {
    use crate::Embedder;

    struct ConstantEmbedder(Vec<f32>);

    impl Embedder for ConstantEmbedder {
        fn embed(&self, _text: &str) -> impl Future<Output = Vec<f32>> + Send {
            let vec = self.0.clone();
            async move { vec }
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[tokio::test]
    async fn embedder_trait_bounds() {
        assert_send_sync::<ConstantEmbedder>();
        let embedder = ConstantEmbedder(vec![0.1, 0.2, 0.3]);
        let result = embedder.embed("hello").await;
        assert_eq!(result, vec![0.1, 0.2, 0.3]);
    }
}
