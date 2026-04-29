//! # Semantic cache for chat completions (Tier-2 #6).
//!
//! Saves token spend by returning a previously-cached response when a
//! new request is *semantically equivalent* to a prior one. Two-tier:
//!
//! 1. **Exact-hash hit** — same canonical request hash as a prior call
//!    → microsecond-fast btree lookup. Catches accidental retries,
//!    UI auto-reloads, idempotency-key replays.
//!
//! 2. **Vector-similarity hit** — when hash misses, embed the prompt
//!    and search the user's prior cached responses for one within
//!    `cache_similarity_threshold` cosine similarity. Catches
//!    paraphrases ("what's the weather" vs "tell me the weather").
//!
//! Cache is opt-in per service and scoped to `(user_id, service_name)`.
//! Different users never share cached LLM responses; different services
//! never share answers across system prompts.
//!
//! Embedding model is OpenAI's `text-embedding-3-small` (1536 dims, the
//! cheapest commodity option at $0.02 / 1M tokens). The embedding
//! provider is configured via `MG_EMBEDDING_API_KEY` (falls back to
//! `MG_OPENAI_API_KEY`). If neither is set, only the exact-hash tier
//! activates — vector search is gracefully skipped.

use anyhow::{anyhow, Result};
use mawi_core::unified::{ChatMessage, UnifiedChatResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;

/// Per-service cache configuration loaded from the `services` table.
/// `enabled = false` short-circuits all of this module.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CacheConfig {
    pub enabled: bool,
    pub similarity_threshold: f32,
    pub ttl_seconds: i32,
}

impl CacheConfig {
    /// Read the cache settings for `service_name` from the DB. Returns
    /// `enabled=false` if the service doesn't exist (caller treats that
    /// as a cache miss; the executor will then hit the same "service
    /// not found" path and produce the right error).
    pub async fn load(pool: &PgPool, service_name: &str) -> Result<Self> {
        let row = sqlx::query_as::<_, CacheConfig>(
            "SELECT
                cache_enabled         AS enabled,
                cache_similarity_threshold AS similarity_threshold,
                cache_ttl_seconds     AS ttl_seconds
             FROM services
             WHERE name = $1",
        )
        .bind(service_name)
        .fetch_optional(pool)
        .await?;
        Ok(row.unwrap_or(CacheConfig {
            enabled: false,
            similarity_threshold: 0.95,
            ttl_seconds: 3600,
        }))
    }
}

/// What `lookup` returns. The caller decides what to do with it:
///   - `Hit { response, similarity }` → return to client without
///     calling the upstream LLM. Increment metrics.
///   - `Miss` → proceed to executor, then call `store` with the result.
pub enum CacheDecision {
    Hit {
        response: UnifiedChatResponse,
        /// 1.0 = exact hash hit; 0.95..1.0 = vector hit at that similarity.
        similarity: f32,
        /// `request_hash` if the hit came from the hash tier, else None.
        /// Useful for emitting a debug header on the response.
        exact: bool,
    },
    Miss,
}

/// Two-tier lookup:
///   1. Exact request hash.
///   2. Vector similarity (only if `cfg.enabled` and an embedding can
///      be computed — failures here degrade gracefully to a miss
///      rather than an error, so a cache outage never breaks chat).
pub async fn lookup(
    pool: &PgPool,
    user_id: &str,
    service_name: &str,
    request_hash: &str,
    messages: &[ChatMessage],
    cfg: &CacheConfig,
) -> CacheDecision {
    if !cfg.enabled {
        return CacheDecision::Miss;
    }

    // Tier 1: exact hash. Fast path catches the dominant retry case.
    // `query_scalar::<_, Value>` returns Result<Option<Value>>; flatten
    // to Result<Option<Value>>::Ok(Some(_)) before deserialising.
    if let Ok(Some(body)) = sqlx::query_scalar::<_, Value>(
        "UPDATE semantic_cache
            SET hit_count = hit_count + 1,
                last_hit_at = NOW()
          WHERE user_id = $1
            AND service_name = $2
            AND request_hash = $3
            AND expires_at > NOW()
        RETURNING response_body",
    )
    .bind(user_id)
    .bind(service_name)
    .bind(request_hash)
    .fetch_optional(pool)
    .await
    {
        if let Ok(resp) = serde_json::from_value::<UnifiedChatResponse>(body) {
            return CacheDecision::Hit {
                response: resp,
                similarity: 1.0,
                exact: true,
            };
        }
    }

    // Tier 2: vector. Only if we can produce an embedding.
    let embedding = match embed_messages(messages).await {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(error = %e, "skip semantic cache: embedding unavailable");
            return CacheDecision::Miss;
        }
    };

    // pgvector cosine distance: 0 = identical, 2 = opposite.
    // similarity = 1 - distance. We want distance < (1 - threshold).
    let max_distance = 1.0_f32 - cfg.similarity_threshold;

    let row = sqlx::query(
        "UPDATE semantic_cache
            SET hit_count = hit_count + 1,
                last_hit_at = NOW()
          WHERE id = (
              SELECT id FROM semantic_cache
              WHERE user_id = $1
                AND service_name = $2
                AND expires_at > NOW()
                AND embedding IS NOT NULL
                AND (embedding <=> $3::vector) < $4
              ORDER BY embedding <=> $3::vector
              LIMIT 1
          )
        RETURNING response_body, (embedding <=> $3::vector) AS distance",
    )
    .bind(user_id)
    .bind(service_name)
    .bind(vector_to_pg_text(&embedding))
    .bind(max_distance as f64)
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => {
            use sqlx::Row;
            let body: Value = r.try_get("response_body").unwrap_or(Value::Null);
            let distance: f64 = r.try_get("distance").unwrap_or(1.0);
            let similarity = (1.0_f64 - distance).max(0.0) as f32;
            if let Ok(resp) = serde_json::from_value::<UnifiedChatResponse>(body) {
                return CacheDecision::Hit {
                    response: resp,
                    similarity,
                    exact: false,
                };
            }
            CacheDecision::Miss
        }
        Ok(None) => CacheDecision::Miss,
        Err(e) => {
            // Don't fail the request because the cache is unhappy.
            tracing::warn!(error = %e, "semantic cache vector query failed; treating as miss");
            CacheDecision::Miss
        }
    }
}

/// Persist a fresh response. Best-effort: if the embedding API is down
/// or the DB write fails, log and continue — the user already has
/// their response, the only loss is a cache entry.
pub async fn store(
    pool: &PgPool,
    user_id: &str,
    service_name: &str,
    request_hash: &str,
    messages: &[ChatMessage],
    response: &UnifiedChatResponse,
    cfg: &CacheConfig,
) {
    if !cfg.enabled {
        return;
    }

    let prompt_text = join_messages(messages);
    let embedding = match embed_messages(messages).await {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::debug!(error = %e, "semantic cache store: embedding unavailable, hash-only entry");
            None
        }
    };

    let response_body = match serde_json::to_value(response) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "semantic cache store: response not serialisable");
            return;
        }
    };

    let response_tokens = response
        .usage
        .as_ref()
        .map(|u| u.total_tokens)
        .unwrap_or(0);

    // ttl=0 means "never expire". Use a sentinel far-future date.
    let ttl = if cfg.ttl_seconds <= 0 {
        // 100 years from now is "effectively forever" without bumping
        // into Postgres TIMESTAMPTZ overflow.
        Duration::from_secs(60 * 60 * 24 * 365 * 100)
    } else {
        Duration::from_secs(cfg.ttl_seconds as u64)
    };

    let res = sqlx::query(
        "INSERT INTO semantic_cache
            (user_id, service_name, request_hash, prompt_text, embedding,
             response_body, response_tokens, expires_at)
         VALUES
            ($1, $2, $3, $4, $5::vector, $6, $7, NOW() + ($8 || ' seconds')::interval)",
    )
    .bind(user_id)
    .bind(service_name)
    .bind(request_hash)
    .bind(&prompt_text)
    .bind(embedding.as_ref().map(|v| vector_to_pg_text(v)))
    .bind(&response_body)
    .bind(response_tokens)
    .bind(ttl.as_secs() as i64)
    .execute(pool)
    .await;

    if let Err(e) = res {
        tracing::warn!(error = %e, "semantic cache store: insert failed");
    }
}

/// Remove every expired row. Called periodically by a background job.
/// Idempotent — safe to run on multiple replicas.
pub async fn purge_expired(pool: &PgPool) -> Result<u64> {
    let res = sqlx::query("DELETE FROM semantic_cache WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Spawn a background task that runs `purge_expired` every interval.
/// Intended to be called once at gateway boot from `main.rs`.
pub fn spawn_purger(pool: PgPool, interval: Duration) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        // Skip the immediate tick; the table will be empty at boot.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            match purge_expired(&pool).await {
                Ok(n) if n > 0 => tracing::info!(rows = n, "semantic cache: purged expired"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "semantic cache: purge failed"),
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Concatenate the role+content of every message into a single string
/// for embedding. Includes role prefixes so "user: hi" and "system: hi"
/// embed differently — important when systems differ between services.
fn join_messages(messages: &[ChatMessage]) -> String {
    let mut out = String::with_capacity(messages.iter().map(|m| m.content.len() + 16).sum());
    for (i, m) in messages.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&m.role);
        out.push_str(": ");
        out.push_str(&m.content);
    }
    out
}

/// pgvector accepts vectors as either binary or `[1.0, 2.0, ...]` text.
/// We use text because sqlx's parameter binding for the binary `vector`
/// type requires a custom encoder; text is portable across sqlx versions.
fn vector_to_pg_text(v: &[f32]) -> String {
    let mut s = String::with_capacity(v.len() * 12 + 2);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        // Use Display to avoid the locale-dependent `,` decimal separator
        // some serializers emit. f32 Display always uses `.`.
        s.push_str(&x.to_string());
    }
    s.push(']');
    s
}

// ---------------------------------------------------------------------------
// Embedding API wrapper. Calls OpenAI's `/v1/embeddings`. Drop-in
// support for any OpenAI-compatible provider (Azure, llama.cpp, vLLM)
// via `MG_EMBEDDING_API_BASE` override.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    input: String,
    model: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Embed the joined prompt. Errors on missing API key, network failure,
/// or unexpected response shape — the caller turns these into cache misses.
async fn embed_messages(messages: &[ChatMessage]) -> Result<Vec<f32>> {
    let api_key = std::env::var("MG_EMBEDDING_API_KEY")
        .or_else(|_| std::env::var("MG_OPENAI_API_KEY"))
        .map_err(|_| anyhow!("no embedding API key configured (set MG_EMBEDDING_API_KEY)"))?;
    let api_base = std::env::var("MG_EMBEDDING_API_BASE")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = std::env::var("MG_EMBEDDING_MODEL")
        .unwrap_or_else(|_| "text-embedding-3-small".to_string());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let resp = client
        .post(format!("{}/embeddings", api_base.trim_end_matches('/')))
        .bearer_auth(&api_key)
        .json(&EmbeddingRequest {
            input: join_messages(messages),
            model: &model,
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("embedding API {}: {}", status, body));
    }

    let parsed: EmbeddingResponse = resp.json().await?;
    parsed
        .data
        .into_iter()
        .next()
        .map(|d| d.embedding)
        .ok_or_else(|| anyhow!("embedding response had no data"))
}
