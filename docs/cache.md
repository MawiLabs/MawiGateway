# Semantic cache

A two-tier prompt cache that returns previously-computed completions
when a new request is *equivalent* to one MawiGateway has seen before.
Saves token spend on workloads with prompt repetition (chat assistants,
RAG, classification, internal tools). Typical savings: 30–70% of
upstream LLM cost for repetitive workloads, near-zero for adversarial
ones.

## How it works

**Tier 1 — exact-hash match.**
Same canonical request hash as a prior call → microsecond btree lookup
→ instant cache hit. Catches accidental retries, UI auto-reloads, and
idempotency-key replays. Always on when the service has caching
enabled.

**Tier 2 — vector-similarity match.**
When the hash misses, the gateway embeds the prompt with OpenAI's
`text-embedding-3-small` (1536 dims, $0.02 / 1M tokens) and searches
the user's prior cached responses. If any are within
`cache_similarity_threshold` cosine similarity, return that response.
Catches paraphrases ("what's the weather" vs "tell me the weather").

## Per-service configuration

Caching is **off by default**. Opt-in per service — some workloads
shouldn't reuse responses (anything with PII, anything with
non-determinism in the system prompt, anything safety-critical).

| Field | Default | Description |
|-------|---------|-------------|
| `cache_enabled` | `false` | Master switch. |
| `cache_similarity_threshold` | `0.95` | Cosine similarity ≥ this → hit. 1.0 = identical, 0.0 = orthogonal. Lower = higher hit rate, less exact. |
| `cache_ttl_seconds` | `3600` | Entry expiry. `0` = never expire. |

Configure via SQL today:

```sql
UPDATE services
   SET cache_enabled = true,
       cache_similarity_threshold = 0.92,
       cache_ttl_seconds = 7200
 WHERE name = 'text-default';
```

CLI / UI exposure is a follow-up. Once the CLI ships service-update
commands, you'll do:

```bash
mawi services update text-default \
    --cache-enabled \
    --cache-threshold 0.92 \
    --cache-ttl 7200
```

## Embedding provider

The cache calls an embedding API to compute prompt vectors. Configured
via env vars:

```bash
MG_EMBEDDING_API_KEY=sk-...                     # required for vector tier
MG_EMBEDDING_API_BASE=https://api.openai.com/v1 # default
MG_EMBEDDING_MODEL=text-embedding-3-small       # default; matches schema dim
```

Falls back to `MG_OPENAI_API_KEY` if `MG_EMBEDDING_API_KEY` is unset.

If no key is configured, only the **exact-hash** tier activates —
vector search is silently skipped. Cache outages never break chat.

## Storage

Postgres + the `pgvector` extension (auto-installed by migration 032).
Each entry stores:

```
id            uuid
user_id       uuid                  -- never share across users
service_name  text                  -- never share across services
request_hash  text                  -- exact-match key
prompt_text   text                  -- audit + re-embedding
embedding     vector(1536)          -- cosine-search index (HNSW)
response_body jsonb                 -- the cached UnifiedChatResponse
response_tokens int                 -- analytics: tokens saved
hit_count     int
last_hit_at   timestamptz
created_at    timestamptz
expires_at    timestamptz           -- TTL sweep
```

Indexes: HNSW on `embedding`, btree on `(user_id, service_name,
request_hash)`, btree on `expires_at` for the purger.

## Operational

**Purger.** A background tokio task sweeps expired rows hourly. Spawned
from `main.rs`; safe to run on every replica.

**Privacy.** Cache rows are scoped to `(user_id, service_name)`.
A user never sees another user's cached response, and a service with a
different system prompt never returns answers that were generated for
a different service.

**Failure modes.**
- Embedding API down → vector tier disabled, exact-hash tier still
  works → no user impact beyond reduced hit rate.
- Postgres slow on the cache table → cache reads / writes log warnings
  but don't fail the request — the user gets the LLM response, just
  without the savings.
- Bad cache entry (corrupt JSON) → falls through to live LLM call
  rather than returning corrupt data.

**Eviction.** TTL-only today. LRU/LFU eviction is a follow-up — for
now, sizing is bounded by `expires_at`.

## When to enable

Good fits:
- **RAG with stable knowledge** — same question, same answer.
- **Classification / extraction** — small set of common inputs.
- **Internal chatbots** — high prompt overlap.
- **Cost-sensitive batch workloads** — bulk processing of similar items.

Bad fits:
- **Per-user personalization** — every prompt has a unique user
  context, hit rate near zero, you pay embedding cost for nothing.
- **Time-sensitive answers** — "what's the weather right now."
- **Compliance/audit-required workloads** — regulators may not accept
  cached responses.

Tune `cache_similarity_threshold` per service. Start at `0.95`
(strict). Watch the logs for `semantic cache HIT` lines — if hit rate
is 0% and you expect higher, drop to `0.92`. If you see hits that
return wrong answers, raise to `0.97` or `0.99`.
