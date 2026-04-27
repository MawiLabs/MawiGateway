-- Migration 031: idempotency-key cache for retry-safe inference POSTs (#41).
--
-- Stripe-style: client sends `Idempotency-Key: <key>` on a POST. The first
-- request executes, stores the response. Subsequent requests with the
-- same (user_id, key) pair return the stored response without re-executing.
--
-- Composite primary key (user_id, key) so two users may share a key value
-- without collision — the key namespace is per-user, not global.
--
-- request_hash is sha256(method || path || body) so a same-key replay with
-- a different request body is detected and rejected with 409 instead of
-- silently returning a stale response.
--
-- expires_at is a Unix timestamp (BIGINT, matches the rest of the schema).
-- A background sweeper deletes rows where expires_at < now(); index
-- supports that scan.

CREATE TABLE IF NOT EXISTS idempotency_keys (
    user_id        TEXT  NOT NULL,
    key            TEXT  NOT NULL,
    request_hash   TEXT  NOT NULL,
    status_code    INTEGER NOT NULL,
    response_body  BYTEA NOT NULL,
    content_type   TEXT,
    created_at     BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW())::BIGINT),
    expires_at     BIGINT NOT NULL,
    PRIMARY KEY (user_id, key)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_expires ON idempotency_keys(expires_at);
