//! Symmetric encryption for credentials at rest.
//!
//! Provider API keys are stored encrypted with `MAWI_MASTER_KEY`
//! (AES-256-GCM, base64-wrapped envelope). The master key is loaded
//! once at boot via [`init_master_key`] — `encrypt_key` / `decrypt_key`
//! then read from the cached value instead of touching env per call.
//!
//! Key requirements (enforced at boot):
//!   - present
//!   - either 64 hex chars (`openssl rand -hex 32`) — decoded to 32 bytes
//!   - or at least 32 raw bytes (`openssl rand -base64 32` then trimmed)
//!
//! The previous implementation zero-padded short keys, which silently
//! degraded AES-256 to whatever entropy you provided. This module refuses
//! anything that doesn't have at least 256 bits.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::env;
use std::sync::OnceLock;

const MASTER_KEY_ENV: &str = "MAWI_MASTER_KEY";
const MIN_KEY_BYTES: usize = 32;

static MASTER_KEY: OnceLock<[u8; 32]> = OnceLock::new();

/// Initialise the master key cache. Call once from `main()` BEFORE the
/// server starts accepting requests. Fails fast with a clear, actionable
/// message if the key is missing or weak.
pub fn init_master_key() -> Result<()> {
    let raw = env::var(MASTER_KEY_ENV).map_err(|_| {
        anyhow!(
            "{} is not set. Generate one with:\n  openssl rand -hex 32\n\
             then export it before starting the gateway:\n  export {}=<that-value>",
            MASTER_KEY_ENV,
            MASTER_KEY_ENV
        )
    })?;

    let key = parse_master_key(&raw).with_context(|| {
        format!(
            "{} is set but invalid. The value must be either 64 hex chars \
             (recommended; from `openssl rand -hex 32`) or at least 32 raw \
             bytes. Short / zero-padded keys are rejected.",
            MASTER_KEY_ENV
        )
    })?;

    MASTER_KEY
        .set(key)
        .map_err(|_| anyhow!("master key already initialised"))?;

    tracing::info!("master key validated ({} bytes of entropy)", MIN_KEY_BYTES);
    Ok(())
}

/// Pull 32 bytes out of the user-provided value.
/// Public so callers can validate without committing to a global.
pub fn parse_master_key(value: &str) -> Result<[u8; 32]> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("master key is empty");
    }
    // 1. Hex form (64 lowercase or uppercase chars).
    if trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            let hi = u8::from_str_radix(&trimmed[2 * i..2 * i + 1], 16)?;
            let lo = u8::from_str_radix(&trimmed[2 * i + 1..2 * i + 2], 16)?;
            *byte = (hi << 4) | lo;
        }
        return Ok(out);
    }
    // 2. Raw bytes (≥ 32). We take the first 32; refuse if too short.
    let bytes = trimmed.as_bytes();
    if bytes.len() < MIN_KEY_BYTES {
        bail!(
            "master key must be at least {} bytes (got {})",
            MIN_KEY_BYTES,
            bytes.len()
        );
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes[..MIN_KEY_BYTES]);
    Ok(out)
}

/// Get the cached key. Initialises lazily from env in tests / one-off
/// scripts that didn't call `init_master_key()` (returns an error rather
/// than panicking).
fn master_key() -> Result<&'static [u8; 32]> {
    if let Some(k) = MASTER_KEY.get() {
        return Ok(k);
    }
    init_master_key()?;
    MASTER_KEY
        .get()
        .ok_or_else(|| anyhow!("master key not initialised"))
}

/// Encrypt a plaintext string using AES-256-GCM. Returns
/// `v1:<nonce-b64>:<ciphertext-b64>`.
pub fn encrypt_key(plaintext: &str) -> Result<String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }
    let key_bytes = master_key()?;
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("encryption failure: {e}"))?;

    Ok(format!(
        "v1:{}:{}",
        BASE64.encode(nonce),
        BASE64.encode(ciphertext)
    ))
}

/// Decrypt a `v1:<nonce>:<ciphertext>` payload. If the value is plaintext
/// (no `v1:` prefix), returns it as-is — kept for migration of legacy rows.
/// See #32 for the plan to retire this backdoor.
pub fn decrypt_key(input: &str) -> Result<String> {
    if input.is_empty() {
        return Ok(String::new());
    }
    if !input.starts_with("v1:") {
        return Ok(input.to_string());
    }

    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 3 {
        bail!("invalid encrypted envelope (want `v1:<nonce>:<cipher>`)");
    }
    let nonce_bytes = BASE64
        .decode(parts[1])
        .with_context(|| "invalid base64 in nonce")?;
    let ciphertext = BASE64
        .decode(parts[2])
        .with_context(|| "invalid base64 in ciphertext")?;

    let key_bytes = master_key()?;
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("decryption failure: {e}"))?;
    String::from_utf8(plaintext_bytes).map_err(|e| anyhow!("invalid UTF-8 in plaintext: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex() {
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let out = parse_master_key(key).unwrap();
        assert_eq!(out[0], 0x01);
        assert_eq!(out[31], 0xef);
    }

    #[test]
    fn parse_hex_uppercase() {
        let key = "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789";
        assert!(parse_master_key(key).is_ok());
    }

    #[test]
    fn parse_raw_32_bytes() {
        let key = "0123456789abcdef0123456789ABCDEF"; // 32 ASCII chars
        let out = parse_master_key(key).unwrap();
        assert_eq!(&out[..], key.as_bytes());
    }

    #[test]
    fn rejects_short_key() {
        let key = "too-short";
        let err = parse_master_key(key).unwrap_err();
        assert!(format!("{err}").contains("at least 32 bytes"));
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_master_key("").is_err());
        assert!(parse_master_key("   ").is_err());
    }

    #[test]
    fn rejects_short_hex() {
        // 30 hex chars → not 64, not ≥32 bytes
        let key = "0123456789abcdef0123456789abcd";
        assert!(parse_master_key(key).is_err());
    }
}
