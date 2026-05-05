use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm,
    Key, // Or Aes256GcmSiv
    Nonce,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sqlx::PgPool;
use std::env;

/// Minimum acceptable byte length of the master key.
///
/// `openssl rand -hex 32` produces 64 ASCII chars (64 bytes), which
/// passes this check. A user who supplies a short passphrase like
/// "secret" would have been silently zero-padded to 32 bytes by the
/// pre-#31 code — effectively reducing AES-256 to a known-prefix key.
/// Now we reject it at boot with a clear error.
pub const MASTER_KEY_MIN_BYTES: usize = 32;

/// Validates `MG_MASTER_KEY` env var.
///
/// Closes #31. Replaces the previous `.expect()` (which panicked with
/// no actionable message) and silent zero-padding (which weakened
/// AES-256 for any operator who set a short key).
///
/// Call this once at boot, before [`encrypt_key`] / [`decrypt_key`]
/// run. On error, the message tells the operator how to generate a
/// proper key.
pub fn validate_master_key() -> Result<()> {
    let key = env::var("MG_MASTER_KEY").map_err(|_| {
        anyhow!(
            "MG_MASTER_KEY is not set. This key encrypts every provider API key in \
             the database; without it the gateway cannot start. Generate one with:\n\
             \n    openssl rand -hex 32\n\n\
             …and set it in your environment (or .env) before booting. \
             STORE A COPY OFF-MACHINE — losing this key makes every encrypted \
             row in the DB unrecoverable."
        )
    })?;
    if key.as_bytes().len() < MASTER_KEY_MIN_BYTES {
        return Err(anyhow!(
            "MG_MASTER_KEY is too short ({} bytes; need >= {}). The pre-#31 code \
             silently zero-padded short keys, weakening AES-256 to whatever entropy \
             the short input had. Regenerate with:\n\
             \n    openssl rand -hex 32\n",
            key.as_bytes().len(),
            MASTER_KEY_MIN_BYTES
        ));
    }
    Ok(())
}

/// Returns the 32-byte master key derived from `MG_MASTER_KEY`.
///
/// Takes the first 32 bytes of the env value (no padding). Callers
/// must have run [`validate_master_key`] at boot, otherwise this
/// returns an error rather than panicking.
fn master_key_bytes() -> Result<[u8; 32]> {
    let s = env::var("MG_MASTER_KEY")
        .map_err(|_| anyhow!("MG_MASTER_KEY not set (did boot validation run?)"))?;
    let src = s.as_bytes();
    if src.len() < MASTER_KEY_MIN_BYTES {
        return Err(anyhow!(
            "MG_MASTER_KEY is too short ({} bytes; need >= {}); regenerate with `openssl rand -hex 32`",
            src.len(),
            MASTER_KEY_MIN_BYTES
        ));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&src[..32]);
    Ok(out)
}

/// Whether plaintext API keys may be silently accepted by [`decrypt_key`].
///
/// Default: `false`. Set `MG_ALLOW_PLAINTEXT_KEYS=true` only as a temporary
/// escape hatch — for example, when bringing up a new instance against a DB
/// that hasn't yet had [`migrate_plaintext_keys`] run against it. Leaving
/// this on in production re-opens the issue described in #32: any row whose
/// `api_key` column was inserted plaintext (legacy data, accidental insert,
/// SQL injection) is readable by the gateway as if it were a valid key.
fn plaintext_allowed() -> bool {
    matches!(
        env::var("MG_ALLOW_PLAINTEXT_KEYS").as_deref(),
        Ok("true" | "TRUE" | "1")
    )
}

/// Encrypts a plaintext string using AES-256-GCM.
/// Returns a base64 encoded string: "nonce|ciphertext"
pub fn encrypt_key(plaintext: &str) -> Result<String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }

    let key_bytes = master_key_bytes()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failure: {}", e))?;

    let nonce_b64 = BASE64.encode(nonce);
    let cipher_b64 = BASE64.encode(ciphertext);

    // Format: "v1:nonce_b64:cipher_b64"
    Ok(format!("v1:{}:{}", nonce_b64, cipher_b64))
}

/// Decrypts a ciphertext string in format "v1:nonce:ciphertext".
///
/// Inputs without the `v1:` prefix are treated as plaintext and **rejected**
/// by default — see [`plaintext_allowed`] and #32. Run
/// [`migrate_plaintext_keys`] at startup to re-encrypt legacy rows.
pub fn decrypt_key(input: &str) -> Result<String> {
    if input.is_empty() {
        return Ok(String::new());
    }

    if !input.starts_with("v1:") {
        if plaintext_allowed() {
            tracing::warn!(
                "plaintext API key read from DB — re-encrypt by running migrate_plaintext_keys()"
            );
            return Ok(input.to_string());
        }
        return Err(anyhow!(
            "plaintext API key rejected (#32 backdoor): run migrate_plaintext_keys() at boot \
             to re-encrypt, or set MG_ALLOW_PLAINTEXT_KEYS=true as a temporary escape hatch"
        ));
    }

    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 3 {
        return Err(anyhow!("Invalid encrypted format"));
    }

    let nonce_b64 = parts[1];
    let cipher_b64 = parts[2];

    let key_bytes = master_key_bytes()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes = BASE64.decode(nonce_b64)?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = BASE64.decode(cipher_b64)?;

    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failure: {}", e))?;

    let plaintext = String::from_utf8(plaintext_bytes)
        .map_err(|e| anyhow!("Invalid UTF-8 in decrypted key: {}", e))?;

    Ok(plaintext)
}

/// Re-encrypts any plaintext API keys in the `providers` and `models`
/// tables, in place. Idempotent: rows already in `v1:` format are skipped.
///
/// Should be called once at startup, after DB init and before any request
/// can read keys via [`decrypt_key`]. After this returns, the DB contains
/// no plaintext keys, and [`decrypt_key`] can refuse plaintext (its default).
///
/// Returns the count of keys that were rotated. Errors propagate — callers
/// may choose to log-and-continue (today's behaviour) or abort boot.
pub async fn migrate_plaintext_keys(pool: &PgPool) -> Result<usize> {
    let mut rotated = 0_usize;

    // Both tables share the same shape: (id TEXT PRIMARY KEY, api_key TEXT).
    // We hand-roll the loop instead of using a JOIN so each row's encryption
    // failure is isolated — one bad row does not abort the others.
    for table in ["providers", "models"] {
        let select = format!(
            "SELECT id, api_key FROM {} \
             WHERE api_key IS NOT NULL AND api_key <> '' AND api_key NOT LIKE 'v1:%'",
            table
        );
        let rows: Vec<(String, String)> = sqlx::query_as(&select).fetch_all(pool).await?;

        if rows.is_empty() {
            continue;
        }
        tracing::info!(
            table = %table,
            count = rows.len(),
            "re-encrypting plaintext API keys (#32 migration)"
        );

        let update = format!("UPDATE {} SET api_key = $1 WHERE id = $2", table);
        for (id, plaintext) in rows {
            let encrypted = match encrypt_key(&plaintext) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(table = %table, id = %id, error = %e, "encrypt_key failed for row");
                    continue;
                }
            };
            sqlx::query(&update)
                .bind(&encrypted)
                .bind(&id)
                .execute(pool)
                .await?;
            rotated += 1;
        }
    }

    Ok(rotated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// `decrypt_key` and `encrypt_key` read process-wide env vars
    /// (`MG_MASTER_KEY`, `MG_ALLOW_PLAINTEXT_KEYS`). Cargo runs tests
    /// in parallel, so we serialise env-mutating tests through this lock.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const TEST_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var("MG_MASTER_KEY", TEST_KEY);
        env::remove_var("MG_ALLOW_PLAINTEXT_KEYS");

        let pt = "sk-test-1234567890abcdef";
        let ct = encrypt_key(pt).unwrap();
        assert!(ct.starts_with("v1:"), "expected v1: prefix, got {}", ct);
        assert_eq!(decrypt_key(&ct).unwrap(), pt);
    }

    #[test]
    fn decrypt_refuses_plaintext_by_default() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var("MG_MASTER_KEY", TEST_KEY);
        env::remove_var("MG_ALLOW_PLAINTEXT_KEYS");

        let err = decrypt_key("sk-plaintext-leaked-via-direct-insert").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("plaintext API key rejected"),
            "expected refusal, got: {}",
            msg
        );
    }

    #[test]
    fn decrypt_allows_plaintext_when_opted_in() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var("MG_MASTER_KEY", TEST_KEY);
        env::set_var("MG_ALLOW_PLAINTEXT_KEYS", "true");

        let pt = "sk-plaintext-grace-period";
        let result = decrypt_key(pt).unwrap();
        assert_eq!(result, pt);

        env::remove_var("MG_ALLOW_PLAINTEXT_KEYS");
    }

    #[test]
    fn validate_master_key_rejects_missing() {
        let _g = ENV_LOCK.lock().unwrap();
        env::remove_var("MG_MASTER_KEY");
        let err = validate_master_key().unwrap_err();
        assert!(
            err.to_string().contains("MG_MASTER_KEY is not set"),
            "got: {}",
            err
        );
        // Restore for parallel-test sanity.
        env::set_var("MG_MASTER_KEY", TEST_KEY);
    }

    #[test]
    fn validate_master_key_rejects_short() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var("MG_MASTER_KEY", "tooshort");
        let err = validate_master_key().unwrap_err();
        assert!(
            err.to_string().contains("too short"),
            "expected 'too short', got: {}",
            err
        );
        env::set_var("MG_MASTER_KEY", TEST_KEY);
    }

    #[test]
    fn validate_master_key_accepts_64_hex() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var("MG_MASTER_KEY", TEST_KEY);
        validate_master_key().expect("64-hex key should pass");
    }

    #[test]
    fn empty_input_passes_through() {
        let _g = ENV_LOCK.lock().unwrap();
        env::set_var("MG_MASTER_KEY", TEST_KEY);
        env::remove_var("MG_ALLOW_PLAINTEXT_KEYS");

        // Empty inputs are not "plaintext" — they're "no key set". Both
        // call sites (encrypt_key/decrypt_key) treat them as identity.
        assert_eq!(decrypt_key("").unwrap(), "");
        assert_eq!(encrypt_key("").unwrap(), "");
    }
}
