use aes_gcm_siv::{Aes256GcmSiv, Key, Nonce, KeyInit};
use aes_gcm_siv::aead::Aead;
use rand::RngCore;
use rand::rngs::OsRng;

// Function for generating keys
pub fn generate_symmetric_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    // OsRng is a cryptographically secure random number generator for generating keys
    OsRng.fill_bytes(&mut key);
    key
}

/// Encrypts raw file bytes using a provided 32-byte key
/// The parameter `file_content` takes a slice of bytes (`&[u8]`)
pub fn encrypt_file(file_content: &[u8], raw_key: &[u8]) -> Result<Vec<u8>, String> {
    // 1. Load the 32-byte (256-bit) key
    let key = Key::<Aes256GcmSiv>::from_slice(raw_key);
    let cipher = Aes256GcmSiv::new(key);
    
    // 2. Generate a unique nonce (Needs to be exactly 96-bits / 12 bytes)
    // should use the `rand` crate to generate this securely for prod
    let nonce = Nonce::from_slice(b"unique nonce"); 
    
    // 3. Encrypt the file content
    match cipher.encrypt(nonce, file_content) {
        Ok(ciphertext) => Ok(ciphertext),
        Err(_) => Err("Encryption failed".to_string()),
    }
}

/// Decrypts the ciphertext back into the original file's bytes
pub fn decrypt_file(ciphertext: &[u8], raw_key: &[u8]) -> Result<Vec<u8>, String> {
    let key = Key::<Aes256GcmSiv>::from_slice(raw_key);
    let cipher = Aes256GcmSiv::new(key);
    let nonce = Nonce::from_slice(b"unique nonce"); 
    
    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => Err("Decryption failed".to_string()),
    }
}