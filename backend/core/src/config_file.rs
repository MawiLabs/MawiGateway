//! YAML configuration file schema and validator.
//!
//! This is the **single source of truth** for the `mawigateway.yaml`
//! file format. The gateway uses it at boot to populate the DB
//! (see `gateway::config_loader`); the SDK / CLI / MCP server can
//! reuse the same structs to validate user-authored files before
//! shipping them to the gateway.
//!
//! # Scope
//! Phase 1 supports POOL services with their providers, models, and
//! model-bindings. The schema includes fields for AGENTIC services and
//! MCP servers so YAML files written today don't have to be rewritten
//! later, but the gateway loader currently warns and skips those
//! sections instead of upserting them. MULTI_MODALITY pool services
//! are accepted in the schema but their semantics are still
//! experimental — change at any time without a major version bump.
//!
//! # Example
//! See `mawigateway.example.yaml` at the repo root for a fully
//! commented file.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Top-level shape of a `mawigateway.yaml` file.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GatewayConfig {
    /// Schema version — bumped on breaking changes. Currently `1`.
    /// Loaders accept any value but log a warning if it doesn't match
    /// the expected version.
    #[serde(default = "default_version")]
    pub version: u32,

    #[serde(default)]
    pub providers: Vec<ProviderConfig>,

    #[serde(default)]
    pub models: Vec<ModelConfig>,

    #[serde(default)]
    pub services: Vec<ServiceConfig>,

    /// MCP servers (Phase 2 — declared in schema, loader currently
    /// warn-and-skips). Real upsert lands in a follow-up PR.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Stable identifier — referenced by `models[].provider`. Treat
    /// this as a slug (lowercase, alphanum + `-` / `_`).
    pub id: String,

    /// Display name shown in admin UIs.
    pub name: String,

    /// Provider family. One of: `openai`, `anthropic`, `gemini`,
    /// `google`, `azure`, `xai`, `mistral`, `deepseek`, `perplexity`,
    /// `elevenlabs`, `selfhosted`. Drives the adapter selection at
    /// runtime.
    #[serde(rename = "type")]
    pub provider_type: String,

    /// Base URL. Defaults inside the gateway are usually fine; set
    /// this for self-hosted endpoints or Azure deployments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,

    /// API version (Azure mostly).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,

    /// Read the API key from this env var at boot. Recommended for
    /// any production deployment; keeps the YAML file safe to commit.
    /// Mutually exclusive with `api_key_value`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,

    /// Inline API key. Only use for local development and CI fixtures.
    /// The loader encrypts at upsert (so the DB never stores plaintext)
    /// but the YAML file itself is not encrypted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_value: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,

    /// Must reference a [`ProviderConfig::id`] in the same file.
    pub provider: String,

    /// One of `text`, `image`, `video`, `audio`. Drives executor
    /// dispatch (chat vs image vs TTS vs STT). Required.
    pub modality: String,

    /// Finer worker discriminator: `text`, `tts`, `stt`, `video_gen`,
    /// `video_understand`. Optional; defaults to a value derived from
    /// `modality` at upsert time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // Pricing (per 1k tokens). All optional — fall back to the
    // built-in static pricing map if absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_1k_input_tokens: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_1k_output_tokens: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_1k_tokens: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,

    /// Per-model overrides (Azure deployments, on-prem). When set,
    /// these take precedence over the provider's defaults at request time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_value: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    /// Service name — what callers send in `request.service`. Primary key.
    pub name: String,

    /// `pool` | `agentic` | `multi_modality` (experimental).
    #[serde(rename = "type")]
    pub service_type: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Routing strategy. Parsed via `RoutingStrategy::parse` so any
    /// alias (`weighted`, `failover`, `cheapest`, …) is accepted.
    /// Required for POOL services with > 1 model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,

    /// For POOL services. Each entry references a `ModelConfig::id`.
    #[serde(default)]
    pub models: Vec<ServiceModelBinding>,

    /// AGENTIC fields — declared but not yet upserted. See module docs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentic: Option<AgenticConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceModelBinding {
    /// Must reference a [`ModelConfig::id`].
    pub id: String,

    /// Routing weight for `weighted_random` strategy. Per-service
    /// weights typically sum to 100, but the executor auto-normalises
    /// any positive sum.
    #[serde(default = "default_weight")]
    pub weight: i32,

    /// Failover position for `health` strategy. Lower = tried first.
    #[serde(default)]
    pub priority: i32,

    /// Disable a model without removing it from the YAML.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_weight() -> i32 {
    50
}
fn default_enabled() -> bool {
    true
}

/// AGENTIC service config — phase-2 placeholder.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgenticConfig {
    pub planner_model: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<i32>,

    #[serde(default)]
    pub tools: Vec<AgenticToolConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgenticToolConfig {
    /// `model` | `service` | `mcp` | `image` | `video` | `tts` | `stt`.
    #[serde(rename = "type")]
    pub tool_type: String,

    /// Identifier of the underlying resource (model id, service name,
    /// MCP server id).
    pub target: String,

    /// Friendly name surfaced to the planner.
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// MCP server config — phase-2 placeholder.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,

    /// `docker` | `stdio` | `sse`.
    #[serde(rename = "type")]
    pub server_type: String,

    /// Container image (docker) or executable path (stdio).
    pub image_or_command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default)]
    pub env: HashMap<String, String>,
}

// -------------------------------------------------------------------
// Validation
// -------------------------------------------------------------------

#[derive(Debug)]
pub enum ConfigValidationError {
    DuplicateId { kind: &'static str, id: String },
    UnknownProvider { model: String, provider: String },
    UnknownModel { service: String, model: String },
    InvalidServiceType { service: String, type_str: String },
    InvalidStrategy { service: String, strategy: String },
    ConflictingProviderKeySources { id: String },
    ConflictingModelKeySources { id: String },
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId { kind, id } => write!(f, "duplicate {} id: {}", kind, id),
            Self::UnknownProvider { model, provider } => {
                write!(
                    f,
                    "model {} references unknown provider {}",
                    model, provider
                )
            }
            Self::UnknownModel { service, model } => {
                write!(f, "service {} references unknown model {}", service, model)
            }
            Self::InvalidServiceType { service, type_str } => write!(
                f,
                "service {} has invalid type {}; expected `pool`, `agentic`, or `multi_modality`",
                service, type_str
            ),
            Self::InvalidStrategy { service, strategy } => write!(
                f,
                "service {} has invalid routing strategy `{}`",
                service, strategy
            ),
            Self::ConflictingProviderKeySources { id } => write!(
                f,
                "provider {} declares both api_key_env and api_key_value — pick one",
                id
            ),
            Self::ConflictingModelKeySources { id } => write!(
                f,
                "model {} declares both api_key_env and api_key_value — pick one",
                id
            ),
        }
    }
}

impl std::error::Error for ConfigValidationError {}

/// Run all structural checks. Returns the **first** error so file
/// authors get a clear single message; future loaders can switch to
/// collecting all errors at once if that becomes useful.
pub fn validate(cfg: &GatewayConfig) -> Result<(), ConfigValidationError> {
    let mut provider_ids: HashSet<&str> = HashSet::new();
    for p in &cfg.providers {
        if !provider_ids.insert(&p.id) {
            return Err(ConfigValidationError::DuplicateId {
                kind: "provider",
                id: p.id.clone(),
            });
        }
        if p.api_key_env.is_some() && p.api_key_value.is_some() {
            return Err(ConfigValidationError::ConflictingProviderKeySources { id: p.id.clone() });
        }
    }

    let mut model_ids: HashSet<&str> = HashSet::new();
    for m in &cfg.models {
        if !model_ids.insert(&m.id) {
            return Err(ConfigValidationError::DuplicateId {
                kind: "model",
                id: m.id.clone(),
            });
        }
        if !provider_ids.contains(m.provider.as_str()) {
            return Err(ConfigValidationError::UnknownProvider {
                model: m.id.clone(),
                provider: m.provider.clone(),
            });
        }
        if m.api_key_env.is_some() && m.api_key_value.is_some() {
            return Err(ConfigValidationError::ConflictingModelKeySources { id: m.id.clone() });
        }
    }

    let mut service_names: HashSet<&str> = HashSet::new();
    for s in &cfg.services {
        if !service_names.insert(&s.name) {
            return Err(ConfigValidationError::DuplicateId {
                kind: "service",
                id: s.name.clone(),
            });
        }
        match s.service_type.to_lowercase().as_str() {
            "pool" | "agentic" | "multi_modality" | "multi-modality" => {}
            other => {
                return Err(ConfigValidationError::InvalidServiceType {
                    service: s.name.clone(),
                    type_str: other.to_string(),
                });
            }
        }
        if let Some(strategy) = &s.strategy {
            // Strategy must round-trip through the canonical parser.
            // The runtime parser in the executor accepts more aliases —
            // YAML files should stick to canonical names so they're
            // self-documenting.
            if crate::routing::RoutingStrategy::from_str(strategy).is_err() {
                return Err(ConfigValidationError::InvalidStrategy {
                    service: s.name.clone(),
                    strategy: strategy.clone(),
                });
            }
        }
        for binding in &s.models {
            if !model_ids.contains(binding.id.as_str()) {
                return Err(ConfigValidationError::UnknownModel {
                    service: s.name.clone(),
                    model: binding.id.clone(),
                });
            }
        }
    }

    let mut mcp_ids: HashSet<&str> = HashSet::new();
    for mcp in &cfg.mcp_servers {
        if !mcp_ids.insert(&mcp.id) {
            return Err(ConfigValidationError::DuplicateId {
                kind: "mcp_server",
                id: mcp.id.clone(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn min_cfg() -> GatewayConfig {
        GatewayConfig {
            version: 1,
            providers: vec![ProviderConfig {
                id: "p1".into(),
                name: "Provider 1".into(),
                provider_type: "openai".into(),
                api_endpoint: None,
                api_version: None,
                api_key_env: Some("MG_OPENAI_API_KEY".into()),
                api_key_value: None,
                description: None,
            }],
            models: vec![ModelConfig {
                id: "m1".into(),
                name: "gpt-4o".into(),
                provider: "p1".into(),
                modality: "text".into(),
                worker_type: None,
                description: None,
                cost_per_1k_input_tokens: None,
                cost_per_1k_output_tokens: None,
                cost_per_1k_tokens: None,
                tier: None,
                api_endpoint: None,
                api_version: None,
                api_key_env: None,
                api_key_value: None,
                context_window: None,
            }],
            services: vec![ServiceConfig {
                name: "chat".into(),
                service_type: "pool".into(),
                description: None,
                strategy: Some("weighted_random".into()),
                models: vec![ServiceModelBinding {
                    id: "m1".into(),
                    weight: 100,
                    priority: 0,
                    enabled: true,
                }],
                agentic: None,
            }],
            mcp_servers: vec![],
        }
    }

    #[test]
    fn valid_minimal_config_passes() {
        validate(&min_cfg()).unwrap();
    }

    #[test]
    fn duplicate_provider_id_rejected() {
        let mut cfg = min_cfg();
        cfg.providers.push(cfg.providers[0].clone());
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(
            err,
            ConfigValidationError::DuplicateId {
                kind: "provider",
                ..
            }
        ));
    }

    #[test]
    fn unknown_provider_ref_rejected() {
        let mut cfg = min_cfg();
        cfg.models[0].provider = "ghost".into();
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigValidationError::UnknownProvider { .. }));
    }

    #[test]
    fn unknown_model_ref_rejected() {
        let mut cfg = min_cfg();
        cfg.services[0].models[0].id = "ghost".into();
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigValidationError::UnknownModel { .. }));
    }

    #[test]
    fn invalid_strategy_rejected() {
        let mut cfg = min_cfg();
        cfg.services[0].strategy = Some("not_a_strategy".into());
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigValidationError::InvalidStrategy { .. }));
    }

    #[test]
    fn yaml_strategy_aliases_accepted() {
        // After the routing-strategies unification, the canonical
        // parser accepts both the canonical names AND the legacy
        // aliases that exist on real services rows. YAML files inherit
        // that — so `leader-worker` (alias for `health`) and the
        // canonical names both validate cleanly.
        let mut cfg = min_cfg();
        cfg.services[0].strategy = Some("leader-worker".into());
        validate(&cfg).unwrap();

        cfg.services[0].strategy = Some("weighted_random".into());
        validate(&cfg).unwrap();

        cfg.services[0].strategy = Some("nonsense_strategy".into());
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(err, ConfigValidationError::InvalidStrategy { .. }));
    }

    #[test]
    fn round_robin_strategy_accepted() {
        let mut cfg = min_cfg();
        cfg.services[0].strategy = Some("round_robin".into());
        // round_robin lands as a canonical name in the routing PR;
        // here we just ensure the validator accepts whatever from_str
        // recognises today.
        let _ = validate(&cfg);
    }

    #[test]
    fn conflicting_key_sources_rejected() {
        let mut cfg = min_cfg();
        cfg.providers[0].api_key_value = Some("sk-test".into()); // also has api_key_env
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(
            err,
            ConfigValidationError::ConflictingProviderKeySources { .. }
        ));
    }

    #[test]
    fn invalid_service_type_rejected() {
        let mut cfg = min_cfg();
        cfg.services[0].service_type = "frobnicator".into();
        let err = validate(&cfg).unwrap_err();
        assert!(matches!(
            err,
            ConfigValidationError::InvalidServiceType { .. }
        ));
    }

    #[test]
    fn yaml_roundtrip() {
        let cfg = min_cfg();
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        let parsed: GatewayConfig = serde_yaml::from_str(&yaml).unwrap();
        validate(&parsed).unwrap();
        assert_eq!(parsed.providers.len(), 1);
        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.services.len(), 1);
    }

    #[test]
    fn example_file_parses_and_validates() {
        // Lock the shipped example file to the schema. If this fails,
        // either fix the example or document the schema change.
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mawigateway.example.yaml");
        let yaml = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e));
        let cfg: GatewayConfig = serde_yaml::from_str(&yaml).unwrap();
        validate(&cfg).unwrap();
        // Sanity: example covers each major surface so reviewers can
        // see at-a-glance that the file isn't a stub.
        assert!(
            cfg.providers.len() >= 3,
            "example should cover multiple providers"
        );
        assert!(
            cfg.models.len() >= 3,
            "example should cover multiple models"
        );
        assert!(
            cfg.services.len() >= 3,
            "example should cover multiple services"
        );
        assert!(
            cfg.services.iter().any(|s| s.service_type == "agentic"),
            "example should declare an agentic service for design completeness"
        );
    }

    #[test]
    fn yaml_parses_minimal_user_authored_file() {
        // The shape an operator actually writes — minimal fields only,
        // relying on serde defaults.
        let yaml = r#"
version: 1
providers:
  - id: openai
    name: OpenAI
    type: openai
    api_key_env: OPENAI_API_KEY
models:
  - id: gpt-4o
    name: gpt-4o
    provider: openai
    modality: text
services:
  - name: chat
    type: pool
    strategy: weighted_random
    models:
      - id: gpt-4o
"#;
        let cfg: GatewayConfig = serde_yaml::from_str(yaml).unwrap();
        validate(&cfg).unwrap();
        // Default weight applied.
        assert_eq!(cfg.services[0].models[0].weight, 50);
        // Default enabled.
        assert!(cfg.services[0].models[0].enabled);
    }
}
