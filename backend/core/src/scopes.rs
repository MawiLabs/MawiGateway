//! API key scopes (#78).
//!
//! Each API key carries a list of scope strings. The auth middleware
//! attaches the validated key's scopes to the request context; each
//! handler declares the scope it requires; the [`is_satisfied_by`]
//! helper does the implication check.
//!
//! ## Implication rules
//!
//! - `admin` implies every other scope.
//! - `chat` implies every `chat:<service>` (operator can call any
//!   service through chat completions).
//! - `chat:<service>` only matches itself (per-service-scoped keys
//!   can ONLY call the named service).
//! - `read` implies `config:read` (a read-only key can read configs).
//! - Everything else is exact-match.
//!
//! ## Why this lives in mawi-core
//!
//! Both the gateway middleware and the auth utils need to do the
//! check, and the CLI/MCP also want to validate scope strings before
//! sending them to the API. Keeping the logic in `mawi-core` means
//! every consumer agrees on what `chat:gpt-4o` means.

use serde::{Deserialize, Serialize};

/// The full set of predefined "well-known" scopes. Anything not on
/// this list is rejected by [`validate_scope`] *unless* it matches the
/// `chat:<service-name>` pattern.
pub const ADMIN: &str = "admin";
pub const READ: &str = "read";
pub const CHAT: &str = "chat";
pub const CONFIG_READ: &str = "config:read";
pub const CONFIG_WRITE: &str = "config:write";

/// Convenience set for "list me every well-known scope," handy for the
/// API key creation UI's scope picker. `chat:<service>` style scopes
/// aren't enumerated — those are typed in by the user.
pub const WELL_KNOWN: &[&str] = &[ADMIN, READ, CHAT, CONFIG_READ, CONFIG_WRITE];

/// Validate a single scope string. Returns Err with a human-readable
/// message if the scope isn't one of the well-known values and doesn't
/// match the `chat:<service-name>` pattern.
pub fn validate_scope(scope: &str) -> Result<(), String> {
    if WELL_KNOWN.contains(&scope) {
        return Ok(());
    }
    if let Some(service) = scope.strip_prefix("chat:") {
        // Service name regex: lowercase alphanumeric, dot, underscore,
        // hyphen. Same charset we accept for service aliases (#90) so
        // `chat:gpt-4o` works whether `gpt-4o` is a service name or
        // an alias of one.
        if service.is_empty() {
            return Err("scope 'chat:' must be followed by a service name".to_string());
        }
        if !service
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(format!(
                "scope '{}' contains invalid characters (allowed: a-z 0-9 . _ -)",
                scope
            ));
        }
        if service.len() > 64 {
            return Err(format!("scope '{}' is too long (service name max 64 chars)", scope));
        }
        return Ok(());
    }
    Err(format!(
        "unknown scope '{}'. Valid: admin, read, chat, chat:<service>, config:read, config:write",
        scope
    ))
}

/// Validate an entire scope list. Errors out with the first invalid
/// entry so the API surfaces ONE clear error rather than a list.
pub fn validate_scopes(scopes: &[String]) -> Result<(), String> {
    if scopes.is_empty() {
        return Err("scope list cannot be empty (use 'admin' for full access)".to_string());
    }
    for s in scopes {
        validate_scope(s)?;
    }
    Ok(())
}

/// Does the granted scope set imply the required scope? Use the
/// implication rules in the module doc comment.
///
/// `granted` is the scope list attached to the API key (loaded by the
/// auth middleware). `required` is the scope the handler declares it
/// needs (e.g. `chat:my-service` for a service-scoped chat call).
pub fn is_satisfied_by(required: &str, granted: &[String]) -> bool {
    // Fast path: admin grants everything.
    if granted.iter().any(|s| s == ADMIN) {
        return true;
    }
    // Direct match.
    if granted.iter().any(|s| s == required) {
        return true;
    }
    // `chat` grants every `chat:<service>` call.
    if let Some(_service) = required.strip_prefix("chat:") {
        if granted.iter().any(|s| s == CHAT) {
            return true;
        }
    }
    // `read` grants `config:read`.
    if required == CONFIG_READ && granted.iter().any(|s| s == READ) {
        return true;
    }
    false
}

/// Required-scope hint used when a handler decides what it needs.
/// Wrapped as a struct so handler code reads `Required::chat()` etc.
/// rather than passing magic strings around.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Required(pub String);

impl Required {
    pub fn admin() -> Self { Self(ADMIN.into()) }
    pub fn read() -> Self { Self(READ.into()) }
    pub fn chat() -> Self { Self(CHAT.into()) }
    pub fn chat_service(service: &str) -> Self { Self(format!("chat:{}", service)) }
    pub fn config_read() -> Self { Self(CONFIG_READ.into()) }
    pub fn config_write() -> Self { Self(CONFIG_WRITE.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_implies_everything() {
        let g = vec!["admin".to_string()];
        assert!(is_satisfied_by("read", &g));
        assert!(is_satisfied_by("chat", &g));
        assert!(is_satisfied_by("chat:my-service", &g));
        assert!(is_satisfied_by("config:write", &g));
    }

    #[test]
    fn chat_implies_chat_service() {
        let g = vec!["chat".to_string()];
        assert!(is_satisfied_by("chat", &g));
        assert!(is_satisfied_by("chat:gpt-4o", &g));
        assert!(!is_satisfied_by("read", &g));
    }

    #[test]
    fn chat_service_is_exact_only() {
        let g = vec!["chat:gpt-4o".to_string()];
        assert!(is_satisfied_by("chat:gpt-4o", &g));
        assert!(!is_satisfied_by("chat:claude-3-5-sonnet", &g));
        assert!(!is_satisfied_by("chat", &g));
    }

    #[test]
    fn read_implies_config_read() {
        let g = vec!["read".to_string()];
        assert!(is_satisfied_by("read", &g));
        assert!(is_satisfied_by("config:read", &g));
        assert!(!is_satisfied_by("config:write", &g));
        assert!(!is_satisfied_by("chat", &g));
    }

    #[test]
    fn validates_well_known() {
        assert!(validate_scope("admin").is_ok());
        assert!(validate_scope("read").is_ok());
        assert!(validate_scope("chat").is_ok());
        assert!(validate_scope("config:read").is_ok());
        assert!(validate_scope("config:write").is_ok());
    }

    #[test]
    fn validates_chat_service() {
        assert!(validate_scope("chat:gpt-4o").is_ok());
        assert!(validate_scope("chat:my_service.v2").is_ok());
        assert!(validate_scope("chat:").is_err());
        assert!(validate_scope("chat:UPPERCASE").is_err());
        assert!(validate_scope("chat:has space").is_err());
    }

    #[test]
    fn rejects_unknown() {
        assert!(validate_scope("write").is_err());
        assert!(validate_scope("delete:everything").is_err());
        assert!(validate_scope("").is_err());
    }
}
