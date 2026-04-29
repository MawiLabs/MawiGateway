-- Audit log (#80).
--
-- Append-only record of every mutating action. Required for SOC 2 /
-- ISO 27001 / HIPAA evidence ("who provisioned this provider key?"),
-- incident response ("we had an outage at 3am, what changed?"), and
-- multi-user safety ("who deleted the production service?").
--
-- Distinct from #46 which is about queue-drop reliability of an
-- existing audit channel; this migration introduces the schema and
-- the emission API that the rest of the codebase calls into.

CREATE TABLE audit_log (
    -- gen_random_uuid() is built-in on Postgres 13+ (no extension needed).
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Who. user_id is TEXT to match the rest of the schema (see #009);
    -- nullable for system actions (background jobs, migrations).
    user_id         TEXT REFERENCES users(id) ON DELETE SET NULL,
    org_id          TEXT REFERENCES organizations(id) ON DELETE SET NULL,

    -- What. dotted name with verb at the end:
    --   service.create / service.update / service.delete
    --   provider.create / provider.delete
    --   model.create / model.delete
    --   api_key.create / api_key.revoke
    --   mcp_server.create / mcp_server.delete
    --   config.apply
    action          TEXT NOT NULL,

    -- Resource ref: 'service:<name>' or 'provider:<uuid>' etc. Lets
    -- you find every event for one service across actions.
    resource        TEXT NOT NULL,

    -- Pre/post state. JSONB so the schema can evolve without a
    -- migration and the diff is queryable. NULL on creates/deletes
    -- where one side doesn't exist.
    before_state    JSONB,
    after_state     JSONB,

    -- Forensic: where did this come from?
    ip_address      INET,
    user_agent      TEXT,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for the read patterns we expect:
--   - "show me everything user X did in the last 24h"   → (user_id, created_at desc)
--   - "show every change to service:text-default"        → (resource, created_at desc)
--   - "show every service.delete in the last week"       → (action, created_at desc)
--   - "list audit log paginated"                         → (created_at desc) covering index
CREATE INDEX audit_log_user_created_idx     ON audit_log (user_id, created_at DESC);
CREATE INDEX audit_log_resource_created_idx ON audit_log (resource, created_at DESC);
CREATE INDEX audit_log_action_created_idx   ON audit_log (action, created_at DESC);
CREATE INDEX audit_log_created_idx          ON audit_log (created_at DESC);

-- Append-only enforcement. Reject UPDATE and DELETE so the audit log
-- can't be tampered with even by a compromised admin role. Use a
-- rule rather than a trigger because rules are slightly cheaper
-- (no per-row function call) and the predicate is simply "always
-- block."
--
-- Tradeoff: GDPR data deletion. If a user invokes their right to
-- erasure, the user_id column is already nullable + ON DELETE SET
-- NULL — the user record gets removed and the audit row stays as
-- "deleted user did X." The PII is the user_id reference, not the
-- row content; the action/resource/state are property of the org.
CREATE RULE audit_log_no_update AS
    ON UPDATE TO audit_log DO INSTEAD NOTHING;

CREATE RULE audit_log_no_delete AS
    ON DELETE TO audit_log DO INSTEAD NOTHING;
