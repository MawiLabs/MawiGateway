//! Boot-time YAML configuration loader.
//!
//! When `MG_CONFIG_FILE` is set, `apply_config_file_if_present` is
//! called from `main` after DB init. The file's contents are validated
//! (see `mawi_core::config_file::validate`) and then upserted into the
//! existing tables — providers, models, services, service_models —
//! using `INSERT ... ON CONFLICT (...) DO UPDATE`. Existing API and DB
//! drift survive: this is *additive + idempotent*, not destructive.
//!
//! API keys come from one of two sources, both never landing on disk
//! plaintext beyond the YAML file itself:
//! - `api_key_env: VAR_NAME` — read from process env at boot
//! - `api_key_value: "..."` — inline (dev only)
//!
//! In both cases we encrypt with `mawi_core::security::encrypt_key`
//! before insert, so the DB always carries `v1:`-prefixed ciphertext.
//!
//! AGENTIC service config and MCP server upsert are deferred to a
//! follow-up PR; the loader currently logs a clear warning if a YAML
//! file declares either, then continues with the rest.

use anyhow::{anyhow, Context, Result};
use mawi_core::config_file::{GatewayConfig, ModelConfig, ProviderConfig, ServiceConfig};
use sqlx::PgPool;
use std::path::Path;
use tracing::{debug, info, warn};

/// Load the file at `MG_CONFIG_FILE`, validate, and upsert into `pool`.
/// No-op (and Ok) when the env var is unset.
pub async fn apply_config_file_if_present(pool: &PgPool) -> Result<()> {
    let path = match std::env::var("MG_CONFIG_FILE") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => {
            debug!("MG_CONFIG_FILE unset — skipping YAML config load");
            return Ok(());
        }
    };

    let cfg = load_config(Path::new(&path))
        .with_context(|| format!("failed to load mawigateway config from {}", path))?;

    info!(
        path = %path,
        version = cfg.version,
        providers = cfg.providers.len(),
        models = cfg.models.len(),
        services = cfg.services.len(),
        mcp_servers = cfg.mcp_servers.len(),
        "loaded YAML config; upserting into DB"
    );

    let counts = apply_config(&cfg, pool).await?;
    info!(
        providers = counts.providers,
        models = counts.models,
        services = counts.services,
        service_models = counts.service_models,
        skipped_agentic = counts.skipped_agentic,
        skipped_mcp_servers = counts.skipped_mcp_servers,
        "YAML config applied"
    );
    Ok(())
}

/// Read + parse + validate a YAML file. No DB I/O.
pub fn load_config(path: &Path) -> Result<GatewayConfig> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("could not read {}", path.display()))?;
    let cfg: GatewayConfig = serde_yaml::from_str(&text).with_context(|| "YAML parse failed")?;

    if cfg.version != 1 {
        warn!(
            version = cfg.version,
            "MG_CONFIG_FILE schema version is not 1; loader will treat \
             unknown fields as best-effort"
        );
    }

    mawi_core::config_file::validate(&cfg).map_err(|e| anyhow!("validation failed: {}", e))?;

    Ok(cfg)
}

#[derive(Debug, Default)]
pub struct UpsertCounts {
    pub providers: u64,
    pub models: u64,
    pub services: u64,
    pub service_models: u64,
    pub skipped_agentic: u64,
    pub skipped_mcp_servers: u64,
}

/// Apply a validated config to the DB. Each section uses `INSERT ON
/// CONFLICT … DO UPDATE`, so re-applying the same file is a no-op.
pub async fn apply_config(cfg: &GatewayConfig, pool: &PgPool) -> Result<UpsertCounts> {
    let mut counts = UpsertCounts::default();

    for p in &cfg.providers {
        upsert_provider(p, pool).await?;
        counts.providers += 1;
    }

    for m in &cfg.models {
        upsert_model(m, pool).await?;
        counts.models += 1;
    }

    for s in &cfg.services {
        // AGENTIC services need a planner_model_id, system_prompt, etc.
        // and tool wiring through service_mcp_*. Defer to a follow-up
        // PR; for now we upsert the service row's basic fields and
        // skip the agentic-specific config + tool wiring.
        if s.service_type.eq_ignore_ascii_case("agentic") {
            warn!(
                service = %s.name,
                "AGENTIC service config in YAML — basic row upserted, planner/tool config deferred to a follow-up"
            );
            counts.skipped_agentic += 1;
        }
        upsert_service(s, pool).await?;
        counts.services += 1;

        // Replace the service's model bindings atomically: delete the
        // existing rows for this service, insert the new ones. Cleaner
        // than per-row upsert because the YAML is the source of truth
        // for the binding set, and (model_id, weight, priority) tuples
        // change together.
        let svc_models_written = replace_service_models(&s.name, &s.models, pool).await?;
        counts.service_models += svc_models_written;

        // Re-derive the service's input/output modalities from the
        // models we just bound. Without this the row keeps the
        // default `["text"] -> ["text"]` and the admin UI shows
        // wrong arrows for voice / transcribe / image / video pools.
        if let Err(e) =
            crate::api::compute_and_update_service_capabilities(pool, &s.name).await
        {
            warn!(service = %s.name, error = %e, "could not recompute service modalities");
        }
    }

    if !cfg.mcp_servers.is_empty() {
        warn!(
            count = cfg.mcp_servers.len(),
            "MCP server config in YAML — upsert deferred to a follow-up PR; \
             use the existing /v1/mcp API for now"
        );
        counts.skipped_mcp_servers = cfg.mcp_servers.len() as u64;
    }

    Ok(counts)
}

async fn upsert_provider(p: &ProviderConfig, pool: &PgPool) -> Result<()> {
    let api_key_encrypted =
        encrypt_resolved_key(p.api_key_env.as_deref(), p.api_key_value.as_deref())
            .with_context(|| format!("provider {}", p.id))?;

    sqlx::query(
        "INSERT INTO providers (id, name, provider_type, api_endpoint, api_version, api_key, description) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (id) DO UPDATE SET \
            name = EXCLUDED.name, \
            provider_type = EXCLUDED.provider_type, \
            api_endpoint = COALESCE(EXCLUDED.api_endpoint, providers.api_endpoint), \
            api_version = COALESCE(EXCLUDED.api_version, providers.api_version), \
            api_key = COALESCE(EXCLUDED.api_key, providers.api_key), \
            description = COALESCE(EXCLUDED.description, providers.description)",
    )
    .bind(&p.id)
    .bind(&p.name)
    .bind(&p.provider_type)
    .bind(&p.api_endpoint)
    .bind(&p.api_version)
    .bind(&api_key_encrypted)
    .bind(&p.description)
    .execute(pool)
    .await
    .with_context(|| format!("upsert provider {}", p.id))?;

    Ok(())
}

async fn upsert_model(m: &ModelConfig, pool: &PgPool) -> Result<()> {
    let api_key_encrypted =
        encrypt_resolved_key(m.api_key_env.as_deref(), m.api_key_value.as_deref())
            .with_context(|| format!("model {}", m.id))?;

    let worker_type = m
        .worker_type
        .clone()
        .unwrap_or_else(|| match m.modality.as_str() {
            "image" => "text".to_string(), // images run synchronously, treated as text-shaped
            "video" => "video_gen".to_string(),
            "audio" => "tts".to_string(), // safer default than stt
            _ => "text".to_string(),
        });

    sqlx::query(
        "INSERT INTO models (id, name, provider_id, modality, description, \
            cost_per_1k_input_tokens, cost_per_1k_output_tokens, cost_per_1k_tokens, tier, \
            api_endpoint, api_version, api_key, worker_type, context_window) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) \
         ON CONFLICT (id) DO UPDATE SET \
            name = EXCLUDED.name, \
            provider_id = EXCLUDED.provider_id, \
            modality = EXCLUDED.modality, \
            description = COALESCE(EXCLUDED.description, models.description), \
            cost_per_1k_input_tokens = COALESCE(EXCLUDED.cost_per_1k_input_tokens, models.cost_per_1k_input_tokens), \
            cost_per_1k_output_tokens = COALESCE(EXCLUDED.cost_per_1k_output_tokens, models.cost_per_1k_output_tokens), \
            cost_per_1k_tokens = COALESCE(EXCLUDED.cost_per_1k_tokens, models.cost_per_1k_tokens), \
            tier = COALESCE(EXCLUDED.tier, models.tier), \
            api_endpoint = COALESCE(EXCLUDED.api_endpoint, models.api_endpoint), \
            api_version = COALESCE(EXCLUDED.api_version, models.api_version), \
            api_key = COALESCE(EXCLUDED.api_key, models.api_key), \
            worker_type = EXCLUDED.worker_type, \
            context_window = COALESCE(EXCLUDED.context_window, models.context_window)",
    )
    .bind(&m.id)
    .bind(&m.name)
    .bind(&m.provider)
    .bind(&m.modality)
    .bind(&m.description)
    .bind(m.cost_per_1k_input_tokens.map(|v| v as f32))
    .bind(m.cost_per_1k_output_tokens.map(|v| v as f32))
    .bind(m.cost_per_1k_tokens.map(|v| v as f32))
    .bind(&m.tier)
    .bind(&m.api_endpoint)
    .bind(&m.api_version)
    .bind(&api_key_encrypted)
    .bind(&worker_type)
    .bind(m.context_window)
    .execute(pool)
    .await
    .with_context(|| format!("upsert model {}", m.id))?;

    Ok(())
}

async fn upsert_service(s: &ServiceConfig, pool: &PgPool) -> Result<()> {
    // Normalise service_type to the values existing code already
    // understands (uppercase POOL / AGENTIC / MULTI_MODALITY).
    let lowered = s.service_type.to_lowercase();
    let normalised_type = match lowered.as_str() {
        "pool" => "POOL",
        "agentic" => "AGENTIC",
        "multi_modality" | "multi-modality" => "MULTI_MODALITY",
        _ => lowered.as_str(), // already validated upstream — pass through
    };

    sqlx::query(
        "INSERT INTO services (name, service_type, description, strategy) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (name) DO UPDATE SET \
            service_type = EXCLUDED.service_type, \
            description = COALESCE(EXCLUDED.description, services.description), \
            strategy = EXCLUDED.strategy",
    )
    .bind(&s.name)
    .bind(normalised_type)
    .bind(&s.description)
    .bind(s.strategy.as_deref().unwrap_or("weighted_random"))
    .execute(pool)
    .await
    .with_context(|| format!("upsert service {}", s.name))?;

    Ok(())
}

async fn replace_service_models(
    service_name: &str,
    bindings: &[mawi_core::config_file::ServiceModelBinding],
    pool: &PgPool,
) -> Result<u64> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM service_models WHERE service_name = $1")
        .bind(service_name)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("clearing service_models for {}", service_name))?;

    let mut written = 0u64;
    for (pos, b) in bindings.iter().enumerate() {
        if !b.enabled {
            // Honour the YAML's `enabled: false` by simply not binding —
            // routing won't even know the model exists for this service.
            continue;
        }
        sqlx::query(
            "INSERT INTO service_models (service_name, model_id, position, weight) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(service_name)
        .bind(&b.id)
        .bind(pos as i32 + b.priority)
        .bind(b.weight)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("binding model {} to service {}", b.id, service_name))?;
        written += 1;
    }

    tx.commit().await?;
    Ok(written)
}

/// Resolve `api_key_env` / `api_key_value` to ciphertext. Returns
/// `Ok(None)` when neither is set (provider/model has no key).
fn encrypt_resolved_key(env_name: Option<&str>, inline: Option<&str>) -> Result<Option<String>> {
    if let Some(env_name) = env_name {
        match std::env::var(env_name) {
            Ok(v) if !v.is_empty() => {
                let ct = mawi_core::security::encrypt_key(&v)?;
                Ok(Some(ct))
            }
            _ => {
                warn!(
                    env = env_name,
                    "api_key_env points at unset/empty var — leaving key blank"
                );
                Ok(None)
            }
        }
    } else if let Some(inline) = inline {
        let ct = mawi_core::security::encrypt_key(inline)?;
        Ok(Some(ct))
    } else {
        Ok(None)
    }
}
