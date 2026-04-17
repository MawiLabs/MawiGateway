# Enterprise RBAC & Audit Logs — Design Spec

**Status:** Approved (pending final spec review)
**Date:** 2026-04-17
**Wave:** 1 of 3 (Enterprise platform upgrade)
**Branch:** `feat/enterprise-rbac-audit`
**Author:** Brainstormed with Claude; approved by @akabusiness001

## 1. Goal

Bring MaWi Gateway from single-user per project to enterprise-ready multi-user orgs with role-based access control and a control-plane audit log. This is the foundation every other enterprise feature (SSO, budgets, guardrails governance) depends on.

## 2. Non-goals

- Multi-org membership per user (single-org enforced in Wave 1; schema does not pre-allow it)
- Custom roles / custom permission sets (fixed four-role hierarchy only)
- SSO / SAML / OIDC (Wave 2)
- SCIM user provisioning (Wave 2)
- API-key scopes beyond inherited creator role (Wave 2/3)
- Audit-log export, streaming, or hash-chain tamper-evidence (Wave 2)
- Workspaces / projects inside an org (not planned)
- External authz policy engine (OPA, Cedar) — fixed roles don't need one

## 3. Scope summary

### In scope (Wave 1)
- Four fixed roles: `owner`, `admin`, `developer`, `viewer`
- Org-owned resources (migrating from user-owned)
- Permission matrix enforced via in-process middleware + `require!` macro
- Member invitation flow with email tokens
- Ownership transfer (single-owner invariant)
- Control-plane audit log (mutations only; reads excluded except `provider.key_revealed`)
- Audit log query endpoint with cursor pagination and filters
- Redaction of secrets in audit metadata
- Opt-in retention via `AUDIT_LOG_RETENTION_DAYS` env var
- Frontend permission-gated UI via server-computed permissions array
- Org / members / audit pages in the frontend

## 4. Role model (fixed hierarchy)

Four built-in roles. Permissions hardcoded per role. No user-editable role definitions.

### 4.1 Permission matrix

| Action | owner | admin | developer | viewer |
|---|:---:|:---:|:---:|:---:|
| **Organization** |
| View org settings | ✓ | ✓ | ✓ | ✓ |
| Edit org settings | ✓ | ✓ | — | — |
| Delete org / transfer ownership | ✓ | — | — | — |
| **Members** |
| List members | ✓ | ✓ | ✓ | ✓ |
| Invite / remove members | ✓ | ✓ | — | — |
| Change role (not to/from `owner`) | ✓ | ✓ | — | — |
| Promote to `owner` | ✓ | — | — | — |
| **Billing** |
| View usage & spend | ✓ | ✓ | ✓ | ✓ |
| View invoices | ✓ | ✓ | — | — |
| Manage payment / plan | ✓ | — | — | — |
| **Providers** |
| List | ✓ | ✓ | ✓ | ✓ |
| Create / edit / delete | ✓ | ✓ | — | — |
| Reveal decrypted API key | ✓ | ✓ | — | — |
| **Models** |
| List | ✓ | ✓ | ✓ | ✓ |
| Create / edit / delete | ✓ | ✓ | — | — |
| **Services** |
| List | ✓ | ✓ | ✓ | ✓ |
| Create / edit | ✓ | ✓ | ✓ | — |
| Delete | ✓ | ✓ | — | — |
| Assign models | ✓ | ✓ | ✓ | — |
| Configure guardrails | ✓ | ✓ | — | — |
| **MCP servers** |
| List | ✓ | ✓ | ✓ | ✓ |
| Register / edit / delete | ✓ | ✓ | ✓ | — |
| **LLM calls via gateway** (chat/images/audio/video/playground) | ✓ | ✓ | ✓ | — |
| **API keys** |
| Create own | ✓ | ✓ | ✓ | — |
| Revoke own | ✓ | ✓ | ✓ | ✓ |
| List all org keys | ✓ | ✓ | — | — |
| Revoke any org key | ✓ | ✓ | — | — |
| **Analytics** |
| View own request logs | ✓ | ✓ | ✓ | — |
| View all org logs | ✓ | ✓ | — | — |
| **Audit logs** |
| View | ✓ | ✓ | — | — |
| **Budgets** *(Wave 3 placeholder)* |
| View budgets | ✓ | ✓ | ✓ | ✓ |
| Set / edit budgets | ✓ | ✓ | — | — |

### 4.2 Design choices embedded in the matrix

- **`viewer` cannot make LLM calls.** Preventing silent cost leaks from "read-only" accounts is more important than naming purity.
- **`developer` can register MCP servers** — they're dev tools — but cannot touch provider credentials.
- **`admin` ≠ `owner`.** Admin does everything except delete the org, transfer ownership, and manage billing.
- **`owner` is singular per org.** Enforced in application code; transfer demotes previous owner to `admin` atomically.

## 5. Data model

Migration file: `backend/migrations/031_enterprise_rbac_audit.sql`

### 5.1 Modified tables

```sql
-- Add role
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'owner'
  CHECK (role IN ('owner', 'admin', 'developer', 'viewer'));

-- Resources: add org_id (authoritative owner), keep user_id as creator/provenance
ALTER TABLE providers     ADD COLUMN org_id TEXT REFERENCES organizations(id);
ALTER TABLE models        ADD COLUMN org_id TEXT REFERENCES organizations(id);
ALTER TABLE services      ADD COLUMN org_id TEXT REFERENCES organizations(id);
ALTER TABLE mcp_servers   ADD COLUMN org_id TEXT REFERENCES organizations(id);
ALTER TABLE api_keys      ADD COLUMN org_id TEXT REFERENCES organizations(id);
ALTER TABLE request_logs  ADD COLUMN org_id TEXT REFERENCES organizations(id);

-- Backfill org_id from users.org_id via user_id
UPDATE providers p SET org_id = u.org_id FROM users u WHERE p.user_id = u.id;
-- (same pattern for models, services, mcp_servers, api_keys, request_logs)

-- Enforce NOT NULL + index
ALTER TABLE providers ALTER COLUMN org_id SET NOT NULL;
CREATE INDEX idx_providers_org_id ON providers(org_id);
-- (same for each table)
```

`user_id` columns stay as-is — documented meaning changes to "creator/provenance". Every query from now on filters by `org_id`, not `user_id`.

### 5.2 New tables

```sql
CREATE TABLE invitations (
  id         TEXT PRIMARY KEY,
  org_id     TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  email      TEXT NOT NULL,
  role       TEXT NOT NULL CHECK (role IN ('admin', 'developer', 'viewer')),
  invited_by TEXT NOT NULL REFERENCES users(id),
  token      TEXT NOT NULL UNIQUE,
  expires_at BIGINT NOT NULL,
  accepted_at BIGINT,
  revoked_at  BIGINT,
  created_at BIGINT NOT NULL
);
CREATE INDEX idx_invitations_org ON invitations(org_id);
CREATE INDEX idx_invitations_email ON invitations(email);

CREATE TABLE audit_logs (
  id               TEXT PRIMARY KEY,    -- ULID
  org_id           TEXT NOT NULL REFERENCES organizations(id),
  actor_user_id    TEXT REFERENCES users(id),
  actor_api_key_id TEXT REFERENCES api_keys(id),
  actor_ip         TEXT,
  actor_user_agent TEXT,
  action           TEXT NOT NULL,
  resource_type    TEXT NOT NULL,
  resource_id      TEXT,
  metadata         JSONB NOT NULL DEFAULT '{}',
  created_at       BIGINT NOT NULL
);
CREATE INDEX idx_audit_logs_org_time ON audit_logs(org_id, created_at DESC);
CREATE INDEX idx_audit_logs_actor ON audit_logs(actor_user_id);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
```

## 6. Auth & authorization flow

### 6.1 Principal

```rust
pub struct Principal {
  pub user_id: String,
  pub org_id:  String,
  pub role:    Role,
  pub via:     AuthMethod, // Session | ApiKey { key_id: String }
  pub ip:      Option<String>,
  pub user_agent: Option<String>,
}

pub enum Role { Owner, Admin, Developer, Viewer }
```

### 6.2 Request flow

```
HTTP request
  ├─ AuthMiddleware
  │    • Parse session cookie OR `Authorization: Bearer mawi_...`
  │    • Load user row → Principal { user_id, org_id, role, via, ip, ua }
  │    • Inject into Request::extensions()
  │    • 401 on failure
  │
  ├─ Handler extracts Principal
  │    require!(principal, Permission::ProviderCreate);
  │    // expands to: 403 if !role_has(role, perm)
  │
  ├─ Handler runs query scoped by org_id (OrgScoped helper enforces this)
  │
  └─ On successful mutation:
       emit AuditEvent via channel → AuditWorker → insert audit_logs row
```

### 6.3 Permission enum + policy

New file `backend/core/src/authz.rs`:

```rust
pub enum Permission {
  OrgView, OrgEdit, OrgDelete,
  MemberList, MemberInvite, MemberRemove, MemberChangeRole, MemberPromoteOwner,
  BillingView, BillingInvoices, BillingManage,
  ProviderList, ProviderCreate, ProviderEdit, ProviderDelete, ProviderRevealKey,
  ModelList, ModelCreate, ModelEdit, ModelDelete,
  ServiceList, ServiceCreate, ServiceEdit, ServiceDelete,
  ServiceAssignModels, ServiceConfigGuardrails,
  McpList, McpRegister, McpEdit, McpDelete,
  GatewayInvoke,
  ApiKeyCreateOwn, ApiKeyRevokeOwn, ApiKeyListAll, ApiKeyRevokeAny,
  AnalyticsViewOwn, AnalyticsViewAll,
  AuditView,
  BudgetView, BudgetEdit,
}

pub fn role_has(role: Role, perm: Permission) -> bool {
  // exhaustive match statement generated from §4.1
}

macro_rules! require {
  ($principal:expr, $perm:expr) => {
    if !role_has($principal.role, $perm) {
      return Err(Error::Forbidden($perm));
    }
  };
}
```

### 6.4 Org scoping helper

`OrgScoped<T>` wrapper around SQLx queries; every data-access function takes `org_id` as a required parameter. Review gate: a query that selects from an org-owned table without filtering by `org_id` is a bug. Code review + a clippy-like lint (manual grep in CI for now) catches regressions.

### 6.5 API keys

API keys inherit their creator's role at request time (not frozen at key-creation time). Revoking role permissions immediately reduces the key's capability. Key-specific scoping (`scopes JSONB` column) is deferred to Wave 2.

## 7. API surface

### 7.1 New endpoints

| Method | Path | Permission | Notes |
|---|---|---|---|
| `GET` | `/v1/auth/me` | authenticated | Returns `{user, org, permissions[]}` |
| `GET` | `/v1/org/members` | `MemberList` | Includes role + last_seen |
| `POST` | `/v1/org/members/invite` | `MemberInvite` | Body: `{email, role}`. Rate-limited per org + per email |
| `PATCH` | `/v1/org/members/{user_id}` | `MemberChangeRole` | Body: `{role}`. Promote-to-owner requires `MemberPromoteOwner` |
| `DELETE` | `/v1/org/members/{user_id}` | `MemberRemove` | Cannot remove `owner` |
| `POST` | `/v1/org/transfer-ownership` | `MemberPromoteOwner` | Atomic: target → owner, caller → admin |
| `GET` | `/v1/org/invitations` | `MemberList` | Lists pending / expired / revoked |
| `DELETE` | `/v1/org/invitations/{id}` | `MemberInvite` | Revokes a pending invitation |
| `POST` | `/v1/auth/accept-invitation` | *public (token is credential)* | Body: `{token, password}`. If no user with the invited email exists → create user. If user exists and has no org (orphaned — shouldn't happen) → attach to the invited org. **If user exists and already belongs to a different org** → 409 with `{error: "already_in_org", message: "..."}`; the user must first leave or transfer their current org. (Wave 2 SSO will revisit this via JIT account linking.) |
| `GET` | `/v1/audit-logs` | `AuditView` | Cursor pagination, filters (actor, action, resource, time) |

### 7.2 Changed endpoints

- `POST /v1/auth/register`: sets new user `role='owner'`. Rejects registration with 409 if a valid invitation exists for the email (UI must route to accept-invitation instead).
- All existing CRUD endpoints: add `require!` at the top, switch queries to `WHERE org_id = $1`, emit audit event on successful mutation.
- `GET /v1/providers/{id}`: `api_key` field is populated only when caller has `ProviderRevealKey`. Otherwise `"***"`.

### 7.3 Response shapes

```json
// GET /v1/auth/me
{
  "user":        { "id": "...", "email": "...", "role": "developer" },
  "org":         { "id": "...", "name": "Acme Corp" },
  "permissions": ["provider.list", "service.create", "gateway.invoke", ...]
}

// 403 body (uniform shape)
{
  "error":               "forbidden",
  "required_permission": "provider.create",
  "role":                "developer",
  "message":             "Your role 'developer' cannot perform 'provider.create'. Contact an admin."
}
```

### 7.4 Rate limits & idempotency

- `/v1/org/members/invite`: per-email + per-org in-memory token bucket (Redis in Wave 2)
- `/v1/auth/accept-invitation`: idempotent on token; double-accept returns existing membership
- `/v1/org/transfer-ownership`: `BEGIN; SELECT FOR UPDATE; UPDATE; UPDATE; COMMIT`

## 8. Migration strategy (Approach A — single-shot)

Project is at `v0.1.0 unreleased`, no SLA. Single-shot migration with brief API downtime is the right choice.

### 8.1 Order of operations (inside one transaction)

1. Schema changes (add columns, create tables)
2. Data backfill (role assignment, org_id propagation)
3. Integrity assertions (`DO $$` blocks that `RAISE EXCEPTION` on violation)
4. `NOT NULL` + index application
5. `CHECK` constraints

On any integrity failure: transaction aborts, DB returns to pre-migration state automatically.

### 8.2 Integrity checks (inside the migration)

```sql
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM users WHERE role IS NULL) THEN
    RAISE EXCEPTION 'users with NULL role detected';
  END IF;
  IF EXISTS (SELECT 1 FROM providers WHERE org_id IS NULL) THEN
    RAISE EXCEPTION 'providers with NULL org_id after backfill';
  END IF;
  -- repeat for models, services, mcp_servers, api_keys, request_logs
  IF EXISTS (
    SELECT org_id FROM users WHERE role = 'owner'
    GROUP BY org_id HAVING COUNT(*) != 1
  ) THEN
    RAISE EXCEPTION 'org without exactly one owner detected';
  END IF;
END $$;
```

### 8.3 Rollback

`backend/migrations/rollback_031.sql` (not auto-run):

```sql
ALTER TABLE users DROP COLUMN role;
ALTER TABLE providers DROP COLUMN org_id;
-- ... other resources
DROP TABLE invitations;
DROP TABLE audit_logs;
```

Preserves pre-migration data. Audit events accrued post-deploy are lost, which is acceptable for rollback.

### 8.4 Seed script for local dev

`backend/migrations/seed_rbac_demo.sql` + `cargo run --bin seed` creates:
- Org "Acme Corp"
- Four users (one per role), password `demo1234`
- One pending invitation for invite-flow testing

Never auto-executed.

## 9. Audit logs

### 9.1 Event catalog (Wave 1)

Naming: `<resource>.<action>` dotted lowercase, past-tense. One event per successful mutation.

| Action | Emitted by | `metadata` fields |
|---|---|---|
| `org.updated` | PATCH org | `before`, `after` |
| `org.owner_transferred` | transfer-ownership | `from_user_id`, `to_user_id` |
| `member.invited` | POST invite | `invited_email`, `invited_role`, `invitation_id` |
| `member.invitation_revoked` | DELETE invitation | `invitation_id`, `invited_email` |
| `member.joined` | accept-invitation | `invitation_id`, `assigned_role` |
| `member.removed` | DELETE member | `removed_user_id`, `removed_email`, `previous_role` |
| `member.role_changed` | PATCH member | `target_user_id`, `from_role`, `to_role` |
| `provider.created` / `.updated` / `.deleted` | provider CRUD | `provider_id`, `name`, `provider_type`, `changed_fields` (names only) |
| `provider.key_revealed` | provider reveal | `provider_id` — the one audited "sensitive read" |
| `model.created` / `.updated` / `.deleted` | model CRUD | `model_id`, `name` |
| `service.created` / `.updated` / `.deleted` | service CRUD | `service_name`, `service_type` |
| `service.model_assigned` / `.model_unassigned` | assign-model | `service_name`, `model_id`, `weight`, `priority` |
| `service.guardrails_configured` | guardrails update | `service_name`, `changed_fields` |
| `mcp.registered` / `.updated` / `.deleted` | MCP CRUD | `mcp_server_id`, `name`, `server_type` |
| `api_key.created` | POST api-key | `key_id`, `name` (never the secret) |
| `api_key.revoked` | DELETE api-key | `key_id`, `name`, `revoked_by_self` |
| `auth.login_succeeded` | POST login | `method` |
| `auth.login_failed` | POST login | `reason_code`, `attempted_email` — **only emitted when the email matches an existing user** (so `org_id` is known). Unknown-email attempts are rate-limited at the request layer but not audited; adding them would require a nullable `org_id` and we keep the schema strict. |
| `auth.logout` | POST logout | — |

Gateway invocations (chat/image/audio/video) are NOT audited — they go to `request_logs`. Keeping the two tables separate prevents audit-log blowup.

### 9.2 Redaction

`redact_metadata(&mut serde_json::Value)` (new helper in `core/src/authz.rs` or `core/src/audit.rs`) scrubs:
- Fields named `api_key`, `password`, `secret`, `token`, `authorization`, `cookie`
- Strings matching known secret patterns (`sk-...`, `mawi_...`, bearer-like hex)
- Values longer than 4KB (truncated to 4KB + `"...<truncated>"` suffix)

Every emit site calls this before writing.

### 9.3 Retention

- `AUDIT_LOG_RETENTION_DAYS` env var semantics:
  - **Unset** or **`0`** or **negative** → indefinite retention (no purge)
  - **Positive integer** `N` → a daily background job deletes `audit_logs` rows where `created_at < now - N days`
- The purge job runs at process startup and then every 24h. It logs the deleted row count per run under a `mawi_audit_retention_purge` counter.

### 9.4 Query endpoint semantics

`GET /v1/audit-logs`:
- Default window: last 30 days
- Max page size: 200
- Cursor pagination via ULID of last row (not offset)
- Filters: `actor_user_id`, `action` (exact or prefix `provider.*`), `resource_type`, `resource_id`, `from`, `to`
- Sort: `created_at DESC` (indexed)

Response: see §9 in design review.

### 9.5 Observability & backpressure

- Prometheus counters: `mawi_audit_events_emitted_total{action}`, `mawi_audit_events_dropped_total`
- Channel capacity: 10,000
- **Backpressure diverges from RequestLogger**: on full channel, `try_send` falls back to blocking `send`. Audit loss is a compliance bug; request slowness is recoverable. This is the one intentional difference from existing fire-and-forget workers.

## 10. Frontend

### 10.1 Permission gating pattern

`AuthContext` caches `permissions: Set<Permission>` from `/v1/auth/me`. All gated UI uses:

```tsx
<Can perm="provider.create"><CreateButton /></Can>
```

No hardcoded role checks. Server-side permission enforcement is authoritative; frontend gating is UX only.

### 10.2 New pages

| Route | Visibility | Purpose |
|---|---|---|
| `/org` | owner/admin | Org settings (name, delete, transfer) |
| `/org/members` | owner/admin | Member table, invite, role changes, remove |
| `/invite/accept?token=...` | public | Accept invitation + set password |
| `/audit` | owner/admin | Audit log viewer with filters |

### 10.3 Modified pages

- `TopBar.tsx`: role badge next to email
- `Sidebar.tsx`: hide `/org`, `/audit` for developer/viewer
- `/providers`, `/services`, `/models`, `/mcp`: action buttons wrapped in `<Can>`; viewer mode is read-only
- `/playground`: disabled for viewer with a tooltip
- Every mutation handler: on 403, show toast from structured error body

### 10.4 New components

- `Can.tsx` — permission gate wrapper
- `MembersTable.tsx` — sortable list with inline role editor
- `InviteMemberModal.tsx` — email + role
- `AuditLogTable.tsx` — filters + expandable rows
- `RoleBadge.tsx` — colored pill per role

## 11. Testing strategy

### 11.1 Backend

| Layer | Tool | Coverage |
|---|---|---|
| Permission matrix | `cargo test` | Exhaustive: every (Role, Permission) pair asserted against §4.1 |
| Redaction corpus | `cargo test` | 20+ fixture secrets; all redacted |
| Migration 031 | `testcontainers-rs` + Postgres 15 | Pre-migration fixture → run → post-state assertions |
| Middleware end-to-end | integration test | Parametric: each role × each protected endpoint → expected 200 or 403 |
| Audit emission | integration test | Every mutating endpoint called; exactly one correct audit_log row, redaction applied |
| Invitation flow | integration test | Invite → accept → role applied; revoked/expired rejected |
| Ownership transfer | integration test | Atomic; exactly one owner remains |

### 11.2 Shell scripts

`examples/test_rbac.sh`: register → invite → accept → role change → remove → audit query (live-deployment smoke test).

### 11.3 Frontend

Component tests for `Can` and `MembersTable` with mocked API. Playwright E2E is explicitly deferred to the Wave-3 Testing Uplift sub-project.

### 11.4 CI

`.github/workflows/ci.yml` gains a Postgres service container for the testcontainers-backed migration test. Existing `fmt`/`clippy`/`build`/`test` steps stay.

## 12. Rollout

### 12.1 Branching

- Base: `main` (`f78ecc7` at design time)
- Branch: `feat/enterprise-rbac-audit`
- Merge: squash via PR (per CONTRIBUTING.md conventional commits)
- No feature flag — unreleased project + all-or-nothing migration means a flag would double code paths for no benefit.

### 12.2 Docs updates on this branch

- `README.md`: "Roles & permissions" section
- `docs/self-hosting.md`: inviting users, audit access, retention env var
- `docs/rbac.md` (new): permission matrix, event catalog, endpoint reference
- `CHANGELOG.md` entry under `[Unreleased]`:
  - Added: RBAC, invitations, audit log, retention env var
  - Changed: resources are org-owned, registration assigns `owner`, provider key reveal is role-gated
  - Migration: single-shot 031 with integrity checks

### 12.3 Operator deployment checklist

1. `pg_dump` backup
2. Stop API container
3. Pull new image
4. Start API → migration 031 runs automatically
5. Verify `GET /health`, `GET /v1/auth/me` returns `role: "owner"` for existing users
6. Rollback: if step 4 fails, migration auto-rolls-back; restore previous image

## 13. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Migration backfill mis-assigns `org_id` for user with missing `org_id` | Integrity check `RAISE EXCEPTION` aborts transaction |
| Handler forgets to add `require!` | Parametric middleware test catches it — every protected endpoint must return 403 for roles without permission |
| Handler forgets to add `WHERE org_id = $1` | `OrgScoped` helper makes the filter mandatory; review gate + manual CI grep until a proper lint is in place |
| Audit channel overflows and events are dropped | Backpressure strategy: block instead of drop; counter alerts if saturation occurs |
| Secrets leak into audit metadata | Central `redact_metadata()` with a 20-fixture test corpus |
| Owner account locked out | `POST /v1/org/transfer-ownership` is atomic; CLI fallback documented in ops runbook (Wave 2) |

## 14. Open questions

None at spec-approval time. To be filled in during implementation if discovered.

## 15. Out of scope (deferred to later waves)

- Multi-org per user
- Custom roles
- SSO (SAML/OIDC) + SCIM — **Wave 2**
- API-key scopes — **Wave 2/3**
- Audit export (CSV/NDJSON) + real-time streaming + hash chain — **Wave 2**
- Workspaces / projects inside an org — not planned
- External policy engines (OPA, Cedar) — not needed for fixed roles

## Appendix A — Files touched (estimate)

### New files
- `backend/migrations/031_enterprise_rbac_audit.sql`
- `backend/migrations/rollback_031.sql`
- `backend/migrations/seed_rbac_demo.sql`
- `backend/core/src/authz.rs`
- `backend/core/src/audit.rs`
- `backend/gateway/src/members_api.rs`
- `backend/gateway/src/invitations_api.rs`
- `backend/gateway/src/audit_api.rs`
- `backend/gateway/src/audit_worker.rs`
- `backend/gateway/tests/migration_031.rs`
- `backend/gateway/tests/permissions_matrix.rs`
- `backend/gateway/tests/audit_emission.rs`
- `backend/gateway/tests/invitation_flow.rs`
- `backend/gateway/tests/ownership_transfer.rs`
- `examples/test_rbac.sh`
- `docs/rbac.md`
- `frontend/components/Can.tsx`
- `frontend/components/MembersTable.tsx`
- `frontend/components/InviteMemberModal.tsx`
- `frontend/components/AuditLogTable.tsx`
- `frontend/components/RoleBadge.tsx`
- `frontend/app/org/page.tsx`
- `frontend/app/org/members/page.tsx`
- `frontend/app/invite/accept/page.tsx`
- `frontend/app/audit/page.tsx`

### Modified files
- `backend/gateway/src/main.rs` — register new routes
- `backend/gateway/src/auth_api.rs` — register returns `role`, accept-invitation path, block-on-pending-invite
- `backend/gateway/src/user_api.rs` — `/v1/auth/me` with permissions array
- `backend/gateway/src/api.rs` — add `require!` and `org_id` scoping to all CRUD
- `backend/gateway/src/chat_new.rs` — require `GatewayInvoke`
- `backend/gateway/src/image_*.rs`, `audio.rs`, `video.rs` — require `GatewayInvoke`
- `backend/gateway/src/mcp_api.rs` — require MCP permissions, org scoping
- `backend/gateway/src/executor.rs` — RequestLogger writes `org_id`
- `backend/core/src/auth/mod.rs` — Principal struct, middleware changes
- `backend/core/src/models.rs` — Provider/Model types gain `org_id`
- `backend/core/src/services.rs` — Service type gains `org_id`
- `backend/Cargo.toml` — `ulid` crate for audit IDs, `testcontainers` dev-dep
- `frontend/contexts/AuthContext.ts` — permissions Set + `can()`
- `frontend/components/TopBar.tsx` — role badge
- `frontend/components/Sidebar.tsx` — hide admin-only links
- `frontend/app/providers/page.tsx`, `services/page.tsx`, `models/page.tsx`, `playground/page.tsx` — `<Can>` wrapping
- `.github/workflows/ci.yml` — Postgres service container
- `README.md`, `docs/self-hosting.md`, `CHANGELOG.md`

Estimated LOC: ~3,500–4,500 net new (backend heavier than frontend).
