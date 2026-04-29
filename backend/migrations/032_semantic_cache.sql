-- Semantic cache (Tier-2 #6).
--
-- Stores prior chat completion responses keyed by an embedding of the
-- request prompt, so that semantically equivalent follow-up prompts
-- can return the cached response without re-billing the upstream LLM
-- provider. Saves 30–70% of token spend on workloads with prompt
-- repetition (chat assistants, classification, RAG-grounded answers).
--
-- Two-tier lookup:
--   1. Exact hash match — `request_hash` indexed btree, microsecond
--      query. Same prompt + same params → same hash → instant cache
--      hit. This is the dominant case (autoreloads, accidental retries,
--      retries with idempotency keys).
--   2. Cosine-similarity match — `embedding` indexed via HNSW. When
--      hash misses but the prompt is *semantically* close to a prior
--      one (>= service.cache_similarity_threshold), serve the cached
--      response. Configurable per service so safety-critical services
--      can opt out.
--
-- Cache is per (user_id, service_name): never share an OpenAI response
-- to one customer with another, never confuse responses across
-- services with different system prompts.

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE semantic_cache (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    service_name    TEXT NOT NULL,

    -- Tier-1 lookup: exact match on the canonical request hash.
    request_hash    TEXT NOT NULL,

    -- Audit + re-embedding: the joined prompt text (system + user messages).
    -- Stored as TEXT (no length cap) — Postgres TOASTs large rows automatically.
    prompt_text     TEXT NOT NULL,

    -- Tier-2 lookup: pgvector embedding. 1536 dims matches OpenAI's
    -- text-embedding-3-small (cheapest commodity option). If you swap to
    -- a different embedding model with different dimensionality, you'll
    -- need a new migration that drops and re-creates this column.
    embedding       vector(1536),

    -- The cached UnifiedChatResponse, stored as JSONB so it round-trips
    -- through serde without a schema migration for response shape changes.
    response_body   JSONB NOT NULL,

    -- For analytics: how many tokens this entry SAVED (would-have-cost).
    -- Populated from response.usage.total_tokens at write time.
    response_tokens INT,

    -- Hit counter and last-hit timestamp for LFU/LRU eviction policies
    -- (eviction is a follow-up; for now we rely on TTL).
    hit_count       INT NOT NULL DEFAULT 0,
    last_hit_at     TIMESTAMPTZ,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL
);

-- HNSW index for fast cosine search. Build params chosen for "good
-- recall, modest memory" — tune ef_construction up if hit rate is low.
CREATE INDEX semantic_cache_embedding_idx
    ON semantic_cache USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Hash lookup index for the fast path.
CREATE INDEX semantic_cache_hash_idx
    ON semantic_cache (user_id, service_name, request_hash);

-- Expiration sweep index: WHERE expires_at < NOW() must be cheap so the
-- background purge job doesn't scan the whole table.
CREATE INDEX semantic_cache_expires_idx
    ON semantic_cache (expires_at);

-- ---------------------------------------------------------------------------
-- Service-level cache configuration. Users opt their services in
-- explicitly — caching is OFF by default because some workloads
-- (anything with non-determinism in the system prompt, anything
-- safety-critical, anything with PII) shouldn't reuse responses.
-- ---------------------------------------------------------------------------

ALTER TABLE services
    ADD COLUMN cache_enabled BOOLEAN NOT NULL DEFAULT false,
    -- Cosine SIMILARITY threshold (1.0 = identical embeddings, 0.0 = orthogonal).
    -- Default 0.95: prompts must be very close to be considered a match.
    -- Lower this (e.g. 0.85) for higher hit rate at the cost of less
    -- exact answers; raise it (e.g. 0.99) for near-exact matching only.
    ADD COLUMN cache_similarity_threshold REAL NOT NULL DEFAULT 0.95,
    -- TTL in seconds. Default 1 hour. Set to 0 to disable expiration
    -- (combine with explicit invalidation in your client code).
    ADD COLUMN cache_ttl_seconds INT NOT NULL DEFAULT 3600;
