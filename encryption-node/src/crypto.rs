//! XChaCha20-Poly1305 AEAD with a versioned envelope format.
//!
//! This module owns all symmetric-cipher state in the crate. `SecretKey`
//! is the only carrier of raw key bytes outside the gRPC boundary.

use chacha20poly1305::aead::Aead;
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const ENVELOPE_VERSION_V1: u8 = 0x01;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const HEADER_LEN: usize = 1 + NONCE_LEN;
const KEY_LEN: usize = 32;

/// 256-bit symmetric key. Bytes are zeroized on drop.
///
/// Construct with [`SecretKey::generate`] for fresh randomness or
/// [`SecretKey::from_bytes`] when wrapping material from the gRPC boundary.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretKey([u8; KEY_LEN]);

impl SecretKey {
    /// Draws a fresh key from the OS CSPRNG.
    pub fn generate() -> Self {
        let mut key = [0u8; KEY_LEN];
        OsRng.fill_bytes(&mut key);
        Self(key)
    }

    /// Wraps caller-supplied bytes, rejecting any length other than [`KEY_LEN`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let arr: [u8; KEY_LEN] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength(bytes.len()))?;
        Ok(Self(arr))
    }

    /// Borrows the raw key bytes. Use only at the gRPC boundary.
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

/// Encrypts `plaintext` under `key` and returns a v1 envelope:
/// `version(1) || nonce(24) || ciphertext+tag`.
///
/// The 192-bit nonce is drawn fresh from `OsRng` per call; two encryptions of
/// the same plaintext under the same key produce different envelopes with
/// overwhelming probability.
pub fn encrypt(plaintext: &[u8], key: &SecretKey) -> Result<Vec<u8>, CryptoError> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key.0));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::Encrypt)?;

    let mut envelope = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    envelope.push(ENVELOPE_VERSION_V1);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

/// Decrypts a v1 envelope under `key`.
///
/// Errors:
/// - [`CryptoError::TooShort`] — envelope can't even hold header + tag.
/// - [`CryptoError::UnsupportedVersion`] — version byte is not v1.
/// - [`CryptoError::AuthFailure`] — tag did not verify (wrong key or tampered ciphertext).
pub fn decrypt(envelope: &[u8], key: &SecretKey) -> Result<Vec<u8>, CryptoError> {
    if envelope.len() < HEADER_LEN + TAG_LEN {
        return Err(CryptoError::TooShort {
            len: envelope.len(),
            min: HEADER_LEN + TAG_LEN,
        });
    }
    let version = envelope[0];
    if version != ENVELOPE_VERSION_V1 {
        return Err(CryptoError::UnsupportedVersion(version));
    }
    let nonce = XNonce::from_slice(&envelope[1..HEADER_LEN]);
    let ciphertext = &envelope[HEADER_LEN..];

    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key.0));
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::AuthFailure)
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("invalid key length: expected {KEY_LEN}, got {0}")]
    InvalidKeyLength(usize),
    #[error("envelope too short: {len} < {min}")]
    TooShort { len: usize, min: usize },
    #[error("unsupported envelope version: {0:#04x}")]
    UnsupportedVersion(u8),
    #[error("encryption failed")]
    Encrypt,
    #[error("authentication failure")]
    AuthFailure,
}

#[cfg(test)]
mod tests {
    use super::*;

    // T2: identical (plaintext, key) must produce distinct envelopes.
    // Catches regression to a fixed-nonce cipher.
    #[test]
    fn nonce_is_fresh_per_encryption() {
        let key = SecretKey::generate();
        let plaintext = b"deterministic input";
        let a = encrypt(plaintext, &key).expect("encrypt a");
        let b = encrypt(plaintext, &key).expect("encrypt b");
        assert_ne!(a, b);
    }

    // T3: a single bit flipped in the Poly1305 tag must fail authentication.
    #[test]
    fn tampered_tag_fails_authentication() {
        let key = SecretKey::generate();
        let mut envelope = encrypt(b"payload", &key).expect("encrypt");
        let last = envelope.len() - 1;
        envelope[last] ^= 0x01;
        assert!(matches!(
            decrypt(&envelope, &key),
            Err(CryptoError::AuthFailure)
        ));
    }

    // T4: a single bit flipped in the ciphertext body must fail authentication.
    #[test]
    fn tampered_ciphertext_fails_authentication() {
        let key = SecretKey::generate();
        let plaintext = b"long enough that the ciphertext body is non-empty";
        let mut envelope = encrypt(plaintext, &key).expect("encrypt");
        envelope[HEADER_LEN] ^= 0x01;
        assert!(matches!(
            decrypt(&envelope, &key),
            Err(CryptoError::AuthFailure)
        ));
    }

    // T5: an envelope smaller than `header + tag` must surface as `TooShort`,
    // distinct from authentication failure.
    #[test]
    fn truncated_envelope_is_rejected() {
        let key = SecretKey::generate();
        let too_small = vec![ENVELOPE_VERSION_V1; HEADER_LEN + TAG_LEN - 1];
        assert!(matches!(
            decrypt(&too_small, &key),
            Err(CryptoError::TooShort { .. })
        ));
    }

    // T6: an unrecognized version byte must surface distinctly so future
    // formats can be added without ambiguity at the boundary.
    #[test]
    fn unknown_version_is_rejected() {
        let key = SecretKey::generate();
        let mut envelope = encrypt(b"x", &key).expect("encrypt");
        envelope[0] = 0xFF;
        assert!(matches!(
            decrypt(&envelope, &key),
            Err(CryptoError::UnsupportedVersion(0xFF))
        ));
    }

    // T7: a ciphertext authenticated under one key must not decrypt under another.
    #[test]
    fn wrong_key_fails_authentication() {
        let key = SecretKey::generate();
        let other = SecretKey::generate();
        let envelope = encrypt(b"secret", &key).expect("encrypt");
        assert!(matches!(
            decrypt(&envelope, &other),
            Err(CryptoError::AuthFailure)
        ));
    }

    // T8: `SecretKey::from_bytes` must reject any length other than KEY_LEN
    // so the gRPC boundary cannot smuggle in malformed key material.
    #[test]
    fn from_bytes_rejects_wrong_length() {
        assert!(matches!(
            SecretKey::from_bytes(&[0u8; 16]),
            Err(CryptoError::InvalidKeyLength(16))
        ));
        assert!(matches!(
            SecretKey::from_bytes(&[0u8; 64]),
            Err(CryptoError::InvalidKeyLength(64))
        ));
    }
}
