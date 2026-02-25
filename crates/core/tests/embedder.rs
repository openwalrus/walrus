//! Tests for the Embedder trait.

use walrus_core::Embedder;

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
