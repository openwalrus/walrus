//! Tests for Mistral provider constructors.

use walrus_mistral::{Mistral, endpoint};

#[test]
fn custom_constructor_sets_endpoint() {
    let client = llm::Client::new();
    let custom = "http://localhost:9999/v1/chat/completions";
    let provider = Mistral::custom(client, "test-key", custom).expect("provider");
    assert_eq!(provider.endpoint(), custom);
}

#[test]
fn api_constructor_uses_default_endpoint() {
    let client = llm::Client::new();
    let provider = Mistral::api(client, "test-key").expect("provider");
    assert_eq!(provider.endpoint(), endpoint::MISTRAL);
}
