//! GET /v1/audit — read API for the append-only audit log (#80).
//!
//! Filterable by user, action, resource, and time range. Paginated via
//! `?limit=&offset=` (default 50, max 200) so a long history doesn't
//! blow up a single response.
//!
//! Auth: scopes-aware. Requires `read` (or anything that implies it,
//! per [`mawi_core::scopes`]) so a least-privilege monitoring key can
//! pull audit data without ever being able to create/delete resources.

use mawi_core::audit::AuditEntry;
use mawi_core::auth::utils::AuthScopes;
use mawi_core::scopes;
use poem::web::Data;
use poem::Request;
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiResponse, OpenApi};
use serde::Serialize;
use sqlx::{PgPool, Row};

#[derive(poem_openapi::Tags)]
enum Tags {
    Audit,
}

#[derive(Serialize, poem_openapi::Object)]
pub struct AuditPage {
    /// Page of entries, newest first.
    items: Vec<AuditEntry>,
    /// Total matching rows across all pages — useful for the UI to
    /// render "showing 50 of 12,304". Cheap because the WHERE filters
    /// hit our btree indexes.
    total: i64,
    /// Echo of the limit applied to this request (after clamping).
    limit: i64,
    /// Echo of the offset applied.
    offset: i64,
}

#[derive(ApiResponse)]
enum AuditResponse {
    #[oai(status = 200)]
    Ok(Json<AuditPage>),
    #[oai(status = 401)]
    Unauthorized(Json<mawi_core::api_error::OpenAiErrorResponse>),
    #[oai(status = 403)]
    Forbidden(Json<mawi_core::api_error::OpenAiErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<mawi_core::api_error::OpenAiErrorResponse>),
}

pub struct AuditApi {
    pub pool: PgPool,
}

#[OpenApi]
impl AuditApi {
    /// List audit log entries.
    ///
    /// Filters compose with AND semantics — pass any subset:
    ///   - `user_id`       — only this user's actions
    ///   - `action`        — exact match (e.g. `service.delete`)
    ///   - `resource`      — exact match (e.g. `service:text-default`)
    ///   - `since` / `until` — RFC 3339 timestamps
    ///
    /// Default limit 50, max 200. Offset for pagination.
    #[oai(path = "/audit", method = "get", tag = "Tags::Audit")]
    async fn list(
        &self,
        _pool: Data<&PgPool>,
        req: &Request,
        user_id: Query<Option<String>>,
        action: Query<Option<String>>,
        resource: Query<Option<String>>,
        since: Query<Option<String>>,
        until: Query<Option<String>>,
        limit: Query<Option<i64>>,
        offset: Query<Option<i64>>,
    ) -> AuditResponse {
        // Auth gate: needs `read` (or admin). Implication rules in
        // mawi_core::scopes mean a key with `admin` is auto-allowed.
        let granted = req
            .extensions()
            .get::<AuthScopes>()
            .map(|s| s.0.clone())
            .unwrap_or_default();
        if !scopes::is_satisfied_by(scopes::READ, &granted) {
            return AuditResponse::Forbidden(Json(mawi_core::api_error::OpenAiError::with_code(
                "API key lacks scope to read the audit log. Required: 'read' or 'admin'.",
                mawi_core::api_error::error_type::PERMISSION,
                "insufficient_scope",
            )));
        }

        // Pagination clamping. Match the project-wide pattern: default
        // 50, hard cap 200 (#40).
        let limit_v = limit.0.unwrap_or(50).clamp(1, 200);
        let offset_v = offset.0.unwrap_or(0).max(0);

        // Parse RFC 3339 timestamps. Invalid input → 200 with empty
        // result is too forgiving (operator wonders why their filter
        // does nothing); return 400 explicitly via the OpenAI error
        // shape so the SDK / CLI surfaces the typo.
        let since_dt = match since.0.as_ref() {
            Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
                Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
                Err(_) => {
                    return AuditResponse::InternalError(Json(
                        mawi_core::api_error::OpenAiError::with_code(
                            format!("invalid 'since' timestamp '{}': must be RFC 3339", s),
                            mawi_core::api_error::error_type::INVALID_REQUEST,
                            "invalid_timestamp",
                        ),
                    ))
                }
            },
            None => None,
        };
        let until_dt = match until.0.as_ref() {
            Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
                Ok(dt) => Some(dt.with_timezone(&chrono::Utc)),
                Err(_) => {
                    return AuditResponse::InternalError(Json(
                        mawi_core::api_error::OpenAiError::with_code(
                            format!("invalid 'until' timestamp '{}': must be RFC 3339", s),
                            mawi_core::api_error::error_type::INVALID_REQUEST,
                            "invalid_timestamp",
                        ),
                    ))
                }
            },
            None => None,
        };

        // Build dynamic WHERE clause. Each filter is a `$N` binding so
        // we never concat user input into SQL.
        let mut clauses: Vec<String> = Vec::new();
        let mut args = QueryArgs::default();
        if let Some(u) = user_id.0.as_ref() {
            args.user_id = Some(u.clone());
            clauses.push(format!("user_id = ${}", args.next_idx()));
        }
        if let Some(a) = action.0.as_ref() {
            args.action = Some(a.clone());
            clauses.push(format!("action = ${}", args.next_idx()));
        }
        if let Some(r) = resource.0.as_ref() {
            args.resource = Some(r.clone());
            clauses.push(format!("resource = ${}", args.next_idx()));
        }
        if let Some(d) = since_dt {
            args.since = Some(d);
            clauses.push(format!("created_at >= ${}", args.next_idx()));
        }
        if let Some(d) = until_dt {
            args.until = Some(d);
            clauses.push(format!("created_at <= ${}", args.next_idx()));
        }

        let where_sql = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };

        // Count + page in two queries (one DB round trip each). For an
        // audit table that grows linearly this stays fast as long as
        // the index hits the WHERE clause.
        let count_sql = format!("SELECT COUNT(*) FROM audit_log {}", where_sql);
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(v) = &args.user_id {
            count_q = count_q.bind(v);
        }
        if let Some(v) = &args.action {
            count_q = count_q.bind(v);
        }
        if let Some(v) = &args.resource {
            count_q = count_q.bind(v);
        }
        if let Some(v) = &args.since {
            count_q = count_q.bind(v);
        }
        if let Some(v) = &args.until {
            count_q = count_q.bind(v);
        }
        let total = match count_q.fetch_one(&self.pool).await {
            Ok(n) => n,
            Err(e) => {
                return AuditResponse::InternalError(Json(mawi_core::api_error::OpenAiError::new(
                    format!("audit count: {}", e),
                    mawi_core::api_error::error_type::API,
                )))
            }
        };

        let page_sql = format!(
            "SELECT id, user_id, org_id, action, resource, before_state, after_state,
                    ip_address::text AS ip_address_str, user_agent, created_at
             FROM audit_log
             {}
             ORDER BY created_at DESC
             LIMIT ${} OFFSET ${}",
            where_sql,
            args.next_idx(),
            args.next_idx(),
        );
        let mut page_q = sqlx::query(&page_sql);
        if let Some(v) = &args.user_id {
            page_q = page_q.bind(v);
        }
        if let Some(v) = &args.action {
            page_q = page_q.bind(v);
        }
        if let Some(v) = &args.resource {
            page_q = page_q.bind(v);
        }
        if let Some(v) = &args.since {
            page_q = page_q.bind(v);
        }
        if let Some(v) = &args.until {
            page_q = page_q.bind(v);
        }
        page_q = page_q.bind(limit_v).bind(offset_v);

        let rows = match page_q.fetch_all(&self.pool).await {
            Ok(rs) => rs,
            Err(e) => {
                return AuditResponse::InternalError(Json(mawi_core::api_error::OpenAiError::new(
                    format!("audit page: {}", e),
                    mawi_core::api_error::error_type::API,
                )))
            }
        };

        let items: Vec<AuditEntry> = rows
            .into_iter()
            .map(|r| {
                let id_v: uuid::Uuid = r.get("id");
                let created: chrono::DateTime<chrono::Utc> = r.get("created_at");
                AuditEntry {
                    id: id_v.to_string(),
                    user_id: r.try_get("user_id").ok(),
                    org_id: r.try_get("org_id").ok(),
                    action: r.get("action"),
                    resource: r.get("resource"),
                    before_state: r.try_get("before_state").ok(),
                    after_state: r.try_get("after_state").ok(),
                    ip_address: r.try_get("ip_address_str").ok(),
                    user_agent: r.try_get("user_agent").ok(),
                    // RFC 3339 with timezone designator. Matches the
                    // input format `since` / `until` parse.
                    created_at: created.to_rfc3339(),
                }
            })
            .collect();

        AuditResponse::Ok(Json(AuditPage {
            items,
            total,
            limit: limit_v,
            offset: offset_v,
        }))
    }
}

/// Counter that emits 1, 2, 3, … so the WHERE clause builder picks the
/// right `$N` binding without callers tracking it manually.
#[derive(Default)]
struct QueryArgs {
    idx: i32,
    user_id: Option<String>,
    action: Option<String>,
    resource: Option<String>,
    since: Option<chrono::DateTime<chrono::Utc>>,
    until: Option<chrono::DateTime<chrono::Utc>>,
}

impl QueryArgs {
    fn next_idx(&mut self) -> i32 {
        self.idx += 1;
        self.idx
    }
}
