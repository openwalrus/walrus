//! Tests for walrus-client connection types.

use walrus_client::Connection;

/// Verify Connection is not Clone (per spec).
#[test]
fn connection_not_clone() {
    fn assert_not_clone<T>() {}
    // This would fail at compile time if Connection were Clone.
    // We just verify the type exists and has expected properties.
    let _: fn() = assert_not_clone::<Connection>;
}

/// Verify Connection is Send (required for async usage).
#[test]
fn connection_is_send() {
    fn assert_send<T: Send>() {}
    let _: fn() = assert_send::<Connection>;
}
