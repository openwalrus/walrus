//! Tests for walrus-cli.

use walrus_cli::add;

#[test]
fn it_works() {
    let result = add(2, 2);
    assert_eq!(result, 4);
}
