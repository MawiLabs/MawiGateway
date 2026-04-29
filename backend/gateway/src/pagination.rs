//! Pagination helper for list endpoints (#40).
//!
//! All list endpoints accept `?limit=N&offset=M` query params. Without
//! this, `SELECT … LIMIT 100` was the largest you could ever fetch
//! and `request_logs` (which grows by every request) had no way to
//! page past the most-recent 100 rows. Worse, a few list endpoints
//! had no `LIMIT` at all — a single call could OOM the gateway when
//! the table grew past a few million rows.
//!
//! Defaults: `limit=50`, `offset=0`. Cap: `limit<=200`. Defaults are
//! tuned for typical UI consumption; the cap is the line below which
//! a single page response stays under a few hundred KB even for
//! larger row shapes (request_logs etc).

use poem::Request;

/// Default page size when the client doesn't specify `?limit=`.
pub const DEFAULT_LIMIT: i64 = 50;

/// Hard upper bound on `?limit=`. Higher values clamp here without
/// erroring — better UX than 400 for a request that's "just" greedy.
pub const MAX_LIMIT: i64 = 200;

/// A validated `(limit, offset)` pair ready to bind into a SQL query.
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub limit: i64,
    pub offset: i64,
}

impl Pagination {
    /// Construct from already-parsed `Option<i64>` values — for use
    /// with `Query<Option<i64>>` extractors in poem-openapi handlers.
    pub fn from_parts(limit: Option<i64>, offset: Option<i64>) -> Self {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = offset.unwrap_or(0).max(0);
        Self { limit, offset }
    }

    /// Parse from the `Request`'s raw query string. For handlers that
    /// already take `req: &Request` (the user_api pattern) this is
    /// the cheapest integration — no extra extractor parameters.
    /// Malformed numbers fall back to defaults rather than 400ing,
    /// matching the "be generous in what you accept" stance the
    /// existing handlers take.
    pub fn from_query(req: &Request) -> Self {
        let q = req.uri().query().unwrap_or("");
        let mut limit = None;
        let mut offset = None;
        for pair in q.split('&') {
            let mut it = pair.splitn(2, '=');
            let k = it.next().unwrap_or("");
            let v = it.next().unwrap_or("");
            match k {
                "limit" => limit = v.parse::<i64>().ok(),
                "offset" => offset = v.parse::<i64>().ok(),
                _ => {}
            }
        }
        Self::from_parts(limit, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing() {
        let p = Pagination::from_parts(None, None);
        assert_eq!(p.limit, DEFAULT_LIMIT);
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn limit_clamped_to_max() {
        let p = Pagination::from_parts(Some(10_000), None);
        assert_eq!(p.limit, MAX_LIMIT);
    }

    #[test]
    fn limit_clamped_to_min() {
        let p = Pagination::from_parts(Some(0), None);
        assert_eq!(p.limit, 1);
        let p = Pagination::from_parts(Some(-5), None);
        assert_eq!(p.limit, 1);
    }

    #[test]
    fn negative_offset_floored_to_zero() {
        let p = Pagination::from_parts(None, Some(-100));
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn from_query_parses_both() {
        // Build a Request directly via Poem's builder.
        let req = Request::builder()
            .uri("/x?limit=25&offset=50".parse().unwrap())
            .finish();
        let p = Pagination::from_query(&req);
        assert_eq!(p.limit, 25);
        assert_eq!(p.offset, 50);
    }

    #[test]
    fn from_query_handles_missing() {
        let req = Request::builder().uri("/x".parse().unwrap()).finish();
        let p = Pagination::from_query(&req);
        assert_eq!(p.limit, DEFAULT_LIMIT);
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn from_query_handles_garbage() {
        let req = Request::builder()
            .uri("/x?limit=not-a-number&offset=also-bad".parse().unwrap())
            .finish();
        let p = Pagination::from_query(&req);
        assert_eq!(p.limit, DEFAULT_LIMIT); // fell back, didn't 400
        assert_eq!(p.offset, 0);
    }
}
