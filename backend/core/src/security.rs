use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key // Or Aes256GcmSiv
};
use anyhow::{anyhow, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::env;

/// Encrypts a plaintext string using AES-256-GCM.
/// Returns a base64 encoded string: "nonce|ciphertext"
pub fn encrypt_key(plaintext: &str) -> Result<String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }

    let master_key_str = env::var("MAWI_MASTER_KEY")
        .expect("CRITICAL: MAWI_MASTER_KEY environment variable MUST be set in production. Generate with: openssl rand -hex 32");
        
    // Ensure key is 32 bytes
    let mut key_bytes = [0u8; 32];
    let src_bytes = master_key_str.as_bytes();
    let len = src_bytes.len().min(32);
    key_bytes[..len].copy_from_slice(&src_bytes[..len]);
    
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failure: {}", e))?;
        
    let nonce_b64 = BASE64.encode(nonce);
    let cipher_b64 = BASE64.encode(ciphertext);
    
    // Format: "v1:nonce_b64:cipher_b64"
    Ok(format!("v1:{}:{}", nonce_b64, cipher_b64))
}

/// Decrypts a ciphertext string in format "v1:nonce:ciphertext".
/// If the input doesn't look encrypted (no prefix), returns it as-is (for backward compat during migration).
pub fn decrypt_key(input: &str) -> Result<String> {
    if input.is_empty() {
        return Ok(String::new());
    }

    // Check for version prefix
    if !input.starts_with("v1:") {
        // Assume plaintext for migration
        return Ok(input.to_string());
    }
    
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 3 {
        return Err(anyhow!("Invalid encrypted format"));
    }
    
    let nonce_b64 = parts[1];
    let cipher_b64 = parts[2];
    
    let master_key_str = env::var("MAWI_MASTER_KEY")
        .expect("CRITICAL: MAWI_MASTER_KEY environment variable MUST be set");
        
    let mut key_bytes = [0u8; 32];
    let src_bytes = master_key_str.as_bytes();
    let len = src_bytes.len().min(32);
    key_bytes[..len].copy_from_slice(&src_bytes[..len]);
    
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    
    let nonce_bytes = BASE64.decode(nonce_b64)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = BASE64.decode(cipher_b64)?;
    
    let plaintext_bytes = cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failure: {}", e))?;
        
    let plaintext = String::from_utf8(plaintext_bytes)
        .map_err(|e| anyhow!("Invalid UTF-8 in decrypted key: {}", e))?;
        
    Ok(plaintext)
}
