-- API key scopes (#78).
--
-- Today every key is full-access. That blocks any team that follows
-- least-privilege: a CI pipeline that only needs `mawi config apply`
-- shouldn't be able to revoke other keys; a read-only key for a
-- monitoring dashboard shouldn't be able to mutate services; a
-- per-service key embedded in a customer's app shouldn't be able to
-- call other services.
--
-- Schema: keys carry a `scopes` array. Each scope is one of the
-- documented predefined values (admin / read / chat / config:read /
-- config:write) OR a per-resource scope of the form `chat:<service-name>`.
--
-- Backfill: every existing key gets `{admin}` so today's behavior is
-- preserved exactly. New keys default to whatever the issuer requests
-- (or `{admin}` if they request nothing — same as the old behavior, so
-- the API key creation flow doesn't break before the UI/CLI surface
-- the scope selector).

ALTER TABLE api_keys
    ADD COLUMN scopes TEXT[] NOT NULL DEFAULT '{admin}';

-- GIN index so the auth middleware's `WHERE 'admin' = ANY(scopes)` and
-- `WHERE 'chat:gpt-4o' = ANY(scopes)` checks stay O(log n) instead of
-- a full scan. Critical because every authenticated request hits this.
CREATE INDEX api_keys_scopes_gin_idx ON api_keys USING GIN (scopes);
