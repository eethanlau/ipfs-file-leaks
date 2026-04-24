//! Integration test: the public crypto API round-trips plaintext.
//!
//! Counterpart to the in-file property suite. This one consumes only the
//! re-exported public surface, so it locks the visibility contract.

use encryption_node::crypto::{decrypt, encrypt, SecretKey};

#[test]
fn public_api_round_trip_recovers_plaintext() {
    let key = SecretKey::generate();
    let plaintext = b"integration test payload";
    let envelope = encrypt(plaintext, &key).expect("encrypt");
    let recovered = decrypt(&envelope, &key).expect("decrypt");
    assert_eq!(recovered, plaintext);
}
