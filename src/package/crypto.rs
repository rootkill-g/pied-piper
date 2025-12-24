/// Cryptography module for .pn package encryption
/// 
/// Uses AES-256-GCM for authenticated encryption with associated data (AEAD).
/// This ensures confidentiality, integrity, and authenticity of package contents.

use anyhow::{Context, Result, bail};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};
use rand::RngCore;

/// Size of AES-256 key in bytes
pub const KEY_SIZE: usize = 32;

/// Size of nonce in bytes (96 bits for GCM)
pub const NONCE_SIZE: usize = 12;

/// Generate a random encryption key
pub fn generate_key() -> [u8; KEY_SIZE] {
    let mut key = [0u8; KEY_SIZE];
    rand::rng().fill_bytes(&mut key);
    key
}

/// Derive encryption key from node peer ID
/// 
/// Each node uses its peer ID to derive a unique encryption key.
/// This means packages are encrypted per-node, and can only be
/// decrypted by that specific node.
pub fn derive_key_from_peer_id(peer_id: &str) -> [u8; KEY_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(b"pipernet-encryption-v1:");
    hasher.update(peer_id.as_bytes());
    
    let hash = hasher.finalize();
    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&hash[..KEY_SIZE]);
    key
}

/// Get the network-wide shared encryption key for package distribution
/// 
/// This key is used for encrypting packages for network-wide distribution.
/// All nodes can decrypt packages encrypted with this key.
/// 
/// NOTE: This is a well-known key for MVP/testing. In production, this should be:
/// - Derived from network genesis block or consensus
/// - Rotated periodically
/// - Stored securely (HSM, key management service)
/// - Possibly per-network/per-deployment
pub fn get_network_key() -> [u8; KEY_SIZE] {
    // Derive from a well-known string for network-wide packages
    let mut hasher = Sha256::new();
    hasher.update(b"pipernet-network-key-v1:shared-encryption-for-distribution");
    
    let hash = hasher.finalize();
    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&hash[..KEY_SIZE]);
    key
}

/// Encrypt data with AES-256-GCM
/// 
/// Format: [nonce(12 bytes)][encrypted_data][auth_tag(16 bytes)]
pub fn encrypt(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != KEY_SIZE {
        bail!("Invalid key size: expected {} bytes, got {}", KEY_SIZE, key.len());
    }
    
    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(key)
        .context("Failed to create cipher")?;
    
    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt
    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
    
    // Prepend nonce to ciphertext
    let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    
    Ok(output)
}

/// Decrypt data with AES-256-GCM
/// 
/// Expects format: [nonce(12 bytes)][encrypted_data][auth_tag(16 bytes)]
pub fn decrypt(ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != KEY_SIZE {
        bail!("Invalid key size: expected {} bytes, got {}", KEY_SIZE, key.len());
    }
    
    if ciphertext.len() < NONCE_SIZE {
        bail!("Ciphertext too short: must be at least {} bytes", NONCE_SIZE);
    }
    
    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(key)
        .context("Failed to create cipher")?;
    
    // Extract nonce and encrypted data
    let (nonce_bytes, encrypted_data) = ciphertext.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    // Decrypt
    let plaintext = cipher.decrypt(nonce, encrypted_data)
        .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
    
    Ok(plaintext)
}

/// Compute SHA-256 hash of data
pub fn hash_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    
    let hash = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&hash);
    output
}

/// Verify data integrity using SHA-256
pub fn verify_hash(data: &[u8], expected_hash: &[u8; 32]) -> bool {
    let actual_hash = hash_sha256(data);
    actual_hash == *expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encryption_roundtrip() {
        let key = generate_key();
        let plaintext = b"Hello, PiperNet!";
        
        let ciphertext = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&ciphertext, &key).unwrap();
        
        assert_eq!(plaintext, &decrypted[..]);
    }
    
    #[test]
    fn test_encryption_empty_data() {
        let key = generate_key();
        let plaintext = b"";
        
        let ciphertext = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&ciphertext, &key).unwrap();
        
        assert_eq!(plaintext, &decrypted[..]);
    }
    
    #[test]
    fn test_encryption_large_data() {
        let key = generate_key();
        let plaintext = vec![0u8; 1_000_000]; // 1MB
        
        let ciphertext = encrypt(&plaintext, &key).unwrap();
        let decrypted = decrypt(&ciphertext, &key).unwrap();
        
        assert_eq!(plaintext, decrypted);
    }
    
    #[test]
    fn test_wrong_key_fails() {
        let key1 = generate_key();
        let key2 = generate_key();
        let plaintext = b"Secret message";
        
        let ciphertext = encrypt(plaintext, &key1).unwrap();
        let result = decrypt(&ciphertext, &key2);
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_corrupted_ciphertext_fails() {
        let key = generate_key();
        let plaintext = b"test data";
        
        let mut ciphertext = encrypt(plaintext, &key).unwrap();
        
        // Corrupt the ciphertext
        if let Some(byte) = ciphertext.get_mut(20) {
            *byte ^= 0xFF;
        }
        
        let result = decrypt(&ciphertext, &key);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_truncated_ciphertext_fails() {
        let key = generate_key();
        let plaintext = b"test data";
        
        let ciphertext = encrypt(plaintext, &key).unwrap();
        
        // Truncate ciphertext
        let truncated = &ciphertext[..10];
        let result = decrypt(truncated, &key);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_derive_key_consistency() {
        let peer_id = "12D3KooWTest123";
        let key1 = derive_key_from_peer_id(peer_id);
        let key2 = derive_key_from_peer_id(peer_id);
        
        assert_eq!(key1, key2);
    }
    
    #[test]
    fn test_different_peers_different_keys() {
        let key1 = derive_key_from_peer_id("12D3KooWPeer1");
        let key2 = derive_key_from_peer_id("12D3KooWPeer2");
        
        assert_ne!(key1, key2);
    }
    
    #[test]
    fn test_network_key_consistency() {
        let key1 = get_network_key();
        let key2 = get_network_key();
        
        assert_eq!(key1, key2);
    }
    
    #[test]
    fn test_network_key_length() {
        let key = get_network_key();
        assert_eq!(key.len(), KEY_SIZE);
    }
    
    #[test]
    fn test_network_key_different_from_peer_keys() {
        let network_key = get_network_key();
        let peer_key = derive_key_from_peer_id("12D3KooWTest");
        
        assert_ne!(network_key, peer_key);
    }
    
    #[test]
    fn test_hash_verification() {
        let data = b"test data";
        let hash = hash_sha256(data);
        
        assert!(verify_hash(data, &hash));
        
        let wrong_data = b"wrong data";
        assert!(!verify_hash(wrong_data, &hash));
    }
    
    #[test]
    fn test_hash_deterministic() {
        let data = b"test data";
        let hash1 = hash_sha256(data);
        let hash2 = hash_sha256(data);
        
        assert_eq!(hash1, hash2);
    }
    
    #[test]
    fn test_hash_different_for_different_data() {
        let hash1 = hash_sha256(b"data1");
        let hash2 = hash_sha256(b"data2");
        
        assert_ne!(hash1, hash2);
    }
    
    #[test]
    fn test_generate_key_randomness() {
        let key1 = generate_key();
        let key2 = generate_key();
        
        // Keys should be different (random)
        assert_ne!(key1, key2);
    }
    
    #[test]
    fn test_generate_key_length() {
        let key = generate_key();
        assert_eq!(key.len(), KEY_SIZE);
    }
}

