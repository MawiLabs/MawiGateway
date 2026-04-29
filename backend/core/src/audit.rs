//! Audit log helpers (#80).
//!
//! Append-only event stream of every mutating action. Schema lives
//! in migration 035; this module supplies the typed emission API that
//! handlers call when they create / update / delete a resource.
//!
//! ## Conventions
//!
//! - **action**: dotted name with verb at the end. `service.create`,
//!   `provider.delete`, `api_key.revoke`. The leading segment is the
//!   resource type, the trailing segment is the verb.
//! - **resource**: `<type>:<id>` (`service:text-default`,
//!   `provider:01HK...`). Lets you query every change to one resource
//!   across actions.
//! - **before / after**: JSONB snapshots. Creates fill `after`, deletes
//!   fill `before`, updates fill both. The diff is computed at read
//!   time, not write time, so the storage cost is per-row not per-diff.
//!
//! ## Why fire-and-forget
//!
//! [`emit`] spawns a tokio task and returns immediately so the audit
//! write never blocks the user-visible request. The cost is that an
//! audit row CAN go missing if the process crashes between the user
//! response and the audit insert (this is the gap #46 tracks at the
//! reliability layer). For the level of audit this issue establishes,
//! that's acceptable; tighter guarantees are a follow-up.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::net::IpAddr;

/// One row in the audit log. Structure matches the migration 035
/// schema. `id` and `created_at` come back as strings (UUID hyphenated,
/// timestamp RFC 3339) because `poem_openapi::Object` doesn't natively
/// know how to render `Uuid` / `DateTime`. The wire shape is
/// JSON-friendly either way.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(poem_openapi::Object))]
pub struct AuditEntry {
    pub id: String,
    pub user_id: Option<String>,
    pub org_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub before_state: Option<Value>,
    pub after_state: Option<Value>,
    /// IPv4/IPv6 as a string ("203.0.113.7", "2001:db8::1") so the
    /// shape is JSON-friendly and the consumer doesn't need to parse
    /// `inet`. Null when not captured.
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    /// RFC 3339 timestamp.
    pub created_at: String,
}

/// All the optional context an audit emission can carry. Construct
/// via [`AuditContext::default`] and override what you have. The
/// emission helper handles None-everywhere gracefully — the row is
/// still useful with just (action, resource).
#[derive(Debug, Default, Clone)]
pub struct AuditContext {
    pub user_id: Option<String>,
    pub org_id: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// Emit an audit event. Fire-and-forget — spawns a tokio task and
/// returns immediately so the audit write never blocks the request.
///
/// Failures are logged via `tracing::warn!` and otherwise swallowed,
/// because losing a single audit row should not turn a successful
/// user action into a 500. The tighter "every row, exactly once"
/// guarantee is tracked separately in #46.
pub fn emit(pool: PgPool, action: &'static str, resource: String, ctx: AuditContext) {
    tokio::spawn(async move {
        let res = sqlx::query(
            "INSERT INTO audit_log
                (user_id, org_id, action, resource, before_state, after_state, ip_address, user_agent)
             VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(ctx.user_id)
        .bind(ctx.org_id)
        .bind(action)
        .bind(&resource)
        .bind(ctx.before)
        .bind(ctx.after)
        .bind(ctx.ip_address.map(|ip| ip.to_string()))
        .bind(ctx.user_agent)
        .execute(&pool)
        .await;
        if let Err(e) = res {
            tracing::warn!(action, %resource, error = %e, "audit emit failed");
        }
    });
}

/// Helper: extract IP + user agent from a poem request. Pass the
/// result into [`AuditContext`] when you have a request available.
pub fn forensic_from_request(req: &poem::Request) -> (Option<IpAddr>, Option<String>) {
    // RemoteAddr lookup — poem stashes it in the request metadata.
    // Fall through to None if unavailable (e.g. tests).
    let ip = req
        .remote_addr()
        .as_socket_addr()
        .map(|sa| sa.ip());
    let ua = req
        .headers()
        .get(poem::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    (ip, ua)
}

/// Predefined action names. Keep these as constants (not free-form
/// strings) so a typo in a handler doesn't silently produce
/// un-queryable rows.
pub mod action {
    pub const SERVICE_CREATE: &str = "service.create";
    pub const SERVICE_UPDATE: &str = "service.update";
    pub const SERVICE_DELETE: &str = "service.delete";
    pub const PROVIDER_CREATE: &str = "provider.create";
    pub const PROVIDER_UPDATE: &str = "provider.update";
    pub const PROVIDER_DELETE: &str = "provider.delete";
    pub const MODEL_CREATE: &str = "model.create";
    pub const MODEL_UPDATE: &str = "model.update";
    pub const MODEL_DELETE: &str = "model.delete";
    pub const API_KEY_CREATE: &str = "api_key.create";
    pub const API_KEY_REVOKE: &str = "api_key.revoke";
    pub const MCP_SERVER_CREATE: &str = "mcp_server.create";
    pub const MCP_SERVER_DELETE: &str = "mcp_server.delete";
    pub const CONFIG_APPLY: &str = "config.apply";
}

/// Build a `resource` reference string. Avoids ad-hoc format!() calls
/// that could drift on capitalisation or separator.
pub fn resource(kind: &str, id: &str) -> String {
    format!("{}:{}", kind, id)
}
