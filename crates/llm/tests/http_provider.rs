//! Tests for HttpProvider header construction.

use walrus_llm::HttpProvider;

#[test]
fn bearer_sets_authorization_header() {
    let client = walrus_llm::Client::new();
    let provider = HttpProvider::bearer(client, "test-key", "http://example.com/v1/chat")
        .expect("bearer provider");

    let auth = provider
        .headers()
        .get("authorization")
        .expect("authorization header");
    assert_eq!(auth.to_str().unwrap(), "Bearer test-key");
    assert_eq!(provider.endpoint(), "http://example.com/v1/chat");
}

#[test]
fn no_auth_omits_authorization_header() {
    let client = walrus_llm::Client::new();
    let provider = HttpProvider::no_auth(client, "http://localhost:11434/v1/chat");

    assert!(provider.headers().get("authorization").is_none());
    assert_eq!(provider.endpoint(), "http://localhost:11434/v1/chat");
}

#[test]
fn bearer_sets_content_type_and_accept() {
    let client = walrus_llm::Client::new();
    let provider =
        HttpProvider::bearer(client, "k", "http://example.com").expect("bearer provider");

    let ct = provider
        .headers()
        .get("content-type")
        .expect("content-type");
    assert_eq!(ct.to_str().unwrap(), "application/json");
    let accept = provider.headers().get("accept").expect("accept");
    assert_eq!(accept.to_str().unwrap(), "application/json");
}

#[test]
fn no_auth_sets_content_type_and_accept() {
    let client = walrus_llm::Client::new();
    let provider = HttpProvider::no_auth(client, "http://localhost:8080");

    let ct = provider
        .headers()
        .get("content-type")
        .expect("content-type");
    assert_eq!(ct.to_str().unwrap(), "application/json");
    let accept = provider.headers().get("accept").expect("accept");
    assert_eq!(accept.to_str().unwrap(), "application/json");
}

#[test]
fn custom_header_sets_named_header() {
    let client = walrus_llm::Client::new();
    let provider = HttpProvider::custom_header(client, "x-api-key", "sk-123", "http://example.com")
        .expect("custom header provider");

    let key = provider.headers().get("x-api-key").expect("x-api-key");
    assert_eq!(key.to_str().unwrap(), "sk-123");
    assert!(provider.headers().get("authorization").is_none());
}
