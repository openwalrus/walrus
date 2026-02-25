//! Tests for walrus-client.

use walrus_client::add;

#[test]
fn it_works() {
    let result = add(2, 2);
    assert_eq!(result, 4);
}
