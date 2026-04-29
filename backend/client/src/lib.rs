//! # MawiGateway typed client
//!
//! Hand-rolled Rust client for the MawiGateway API. Lives in the same
//! workspace as the gateway server so when the API surface evolves
//! (`backend/gateway/src/api.rs`, `user_api.rs`, etc.), this crate
//! breaks the build of every downstream consumer (`mawi` CLI,
//! `mawi-mcp` server) — forcing them to update in lockstep.
//!
//! That's the contract: the API server owns the schema, this crate owns
//! the typed view of it, and the CLI + MCP server both consume this
//! crate. To "improve them when the API changes" you only have to
//! touch one file: this one.
//!
//! ## Authentication
//!
//! All endpoints (other than `/auth/*`) require a Bearer token. Pass an
//! API key generated via `POST /v1/user/api-keys` (or the `mawi keys
//! create` CLI command) to [`Client::new`].
//!
//! ## Surface coverage
//!
//! The API has ~47 routes today. This crate covers the high-leverage
//! subset that an AI agent or operator script actually needs:
//! providers, models, services, MCP servers, API keys, logs, analytics,
//! and chat completions. CRUD on the rest of the surface is reachable
//! via [`Client::request`] (escape hatch returning [`serde_json::Value`])
//! so consumers are never blocked on a missing typed wrapper.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{header, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Error type. We surface HTTP status separately from JSON-decode errors so
// CLI and MCP can format failures differently (CLI prints to stderr with a
// non-zero exit; MCP returns an MCP error object the agent can react to).
// ---------------------------------------------------------------------------

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP {status}: {body}")]
    Http { status: StatusCode, body: String },
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("JSON decode error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid response: {0}")]
    Invalid(String),
}

// ---------------------------------------------------------------------------
// The client. Wrap a `reqwest::Client` with the gateway base URL and an
// optional API key. All HTTP methods funnel through `request()` so we have
// a single place to attach headers, set timeouts, and parse errors.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl Client {
    /// Build a client. `base_url` is the gateway origin (e.g.
    /// `http://localhost:8030`). Pass the API key here once; it's
    /// attached to every request.
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            // The gateway does its own connection pooling; we just want
            // sensible per-request timeouts so a hung backend doesn't
            // hang the CLI/MCP forever.
            .build()
            .expect("reqwest client should build with default config");

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            http,
        }
    }

    /// Low-level escape hatch. Use when you need an endpoint this crate
    /// doesn't have a typed wrapper for yet. Returns the raw JSON value
    /// so callers can pluck what they need.
    pub async fn request<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<Value, ClientError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.request(method, &url);

        if let Some(key) = &self.api_key {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", key));
        }

        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(ClientError::Http { status, body: text });
        }

        if text.is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_str(&text).map_err(ClientError::from)
    }

    fn err_context(e: ClientError, ctx: &str) -> anyhow::Error {
        anyhow!("{}: {}", ctx, e)
    }

    // ---- Auth ---------------------------------------------------------

    pub async fn whoami(&self) -> Result<UserMe> {
        let v = self
            .request::<()>(Method::GET, "/v1/user/me", None)
            .await
            .map_err(|e| Self::err_context(e, "GET /v1/user/me"))?;
        serde_json::from_value(v).context("decode UserMe")
    }

    // ---- API keys -----------------------------------------------------

    pub async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        let v = self
            .request::<()>(Method::GET, "/v1/user/api-keys", None)
            .await
            .map_err(|e| Self::err_context(e, "list api keys"))?;
        serde_json::from_value(v).context("decode api keys")
    }

    /// Create an API key. `scopes = None` (or empty) defaults to
    /// `["admin"]` server-side for backwards compatibility. Pass an
    /// explicit list (e.g. `vec!["read".into()]`,
    /// `vec!["chat:gpt-4o".into()]`) for a least-privilege key.
    pub async fn create_api_key(
        &self,
        name: &str,
        scopes: Option<Vec<String>>,
    ) -> Result<CreatedApiKey> {
        let mut body = serde_json::json!({ "name": name });
        if let Some(s) = scopes {
            if !s.is_empty() {
                body["scopes"] = serde_json::json!(s);
            }
        }
        let v = self
            .request(Method::POST, "/v1/user/api-keys", Some(&body))
            .await
            .map_err(|e| Self::err_context(e, "create api key"))?;
        serde_json::from_value(v).context("decode CreatedApiKey")
    }

    pub async fn revoke_api_key(&self, id: &str) -> Result<()> {
        self.request::<()>(Method::DELETE, &format!("/v1/user/api-keys/{}", id), None)
            .await
            .map_err(|e| Self::err_context(e, "revoke api key"))?;
        Ok(())
    }

    // ---- Providers ----------------------------------------------------

    pub async fn list_providers(&self) -> Result<Vec<Provider>> {
        let v = self
            .request::<()>(Method::GET, "/v1/user/providers", None)
            .await
            .map_err(|e| Self::err_context(e, "list providers"))?;
        serde_json::from_value(v).context("decode providers")
    }

    pub async fn create_provider(&self, body: &CreateProvider) -> Result<Provider> {
        let v = self
            .request(Method::POST, "/v1/providers", Some(body))
            .await
            .map_err(|e| Self::err_context(e, "create provider"))?;
        serde_json::from_value(v).context("decode Provider")
    }

    pub async fn delete_provider(&self, id: &str) -> Result<()> {
        self.request::<()>(Method::DELETE, &format!("/v1/providers/{}", id), None)
            .await
            .map_err(|e| Self::err_context(e, "delete provider"))?;
        Ok(())
    }

    // ---- Models -------------------------------------------------------

    pub async fn list_models(&self) -> Result<Vec<Model>> {
        let v = self
            .request::<()>(Method::GET, "/v1/user/models", None)
            .await
            .map_err(|e| Self::err_context(e, "list models"))?;
        serde_json::from_value(v).context("decode models")
    }

    pub async fn create_model(&self, body: &CreateModel) -> Result<Model> {
        let v = self
            .request(Method::POST, "/v1/models", Some(body))
            .await
            .map_err(|e| Self::err_context(e, "create model"))?;
        serde_json::from_value(v).context("decode Model")
    }

    pub async fn delete_model(&self, id: &str) -> Result<()> {
        self.request::<()>(Method::DELETE, &format!("/v1/models/{}", id), None)
            .await
            .map_err(|e| Self::err_context(e, "delete model"))?;
        Ok(())
    }

    // ---- Services -----------------------------------------------------

    pub async fn list_services(&self) -> Result<Vec<Service>> {
        let v = self
            .request::<()>(Method::GET, "/v1/user/services", None)
            .await
            .map_err(|e| Self::err_context(e, "list services"))?;
        serde_json::from_value(v).context("decode services")
    }

    pub async fn create_service(&self, body: &CreateService) -> Result<Service> {
        let v = self
            .request(Method::POST, "/v1/services", Some(body))
            .await
            .map_err(|e| Self::err_context(e, "create service"))?;
        serde_json::from_value(v).context("decode Service")
    }

    pub async fn delete_service(&self, name: &str) -> Result<()> {
        self.request::<()>(Method::DELETE, &format!("/v1/services/{}", name), None)
            .await
            .map_err(|e| Self::err_context(e, "delete service"))?;
        Ok(())
    }

    /// Patch a service. All fields in [`UpdateService`] are optional;
    /// `None` leaves the existing value alone, `Some(v)` replaces it.
    /// `aliases: Some(vec![])` clears the alias list.
    pub async fn update_service(&self, name: &str, body: &UpdateService) -> Result<Service> {
        let v = self
            .request(Method::PUT, &format!("/v1/services/{}", name), Some(body))
            .await
            .map_err(|e| Self::err_context(e, "update service"))?;
        serde_json::from_value(v).context("decode Service")
    }

    // ---- MCP servers --------------------------------------------------

    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServer>> {
        let v = self
            .request::<()>(Method::GET, "/v1/mcp/servers", None)
            .await
            .map_err(|e| Self::err_context(e, "list mcp servers"))?;
        // Endpoint is paginated; expect either `{ items, total }` or a flat array.
        if let Some(items) = v.get("items") {
            return serde_json::from_value(items.clone()).context("decode mcp items");
        }
        serde_json::from_value(v).context("decode mcp servers")
    }

    pub async fn create_mcp_server(&self, body: &CreateMcpServer) -> Result<McpServer> {
        let v = self
            .request(Method::POST, "/v1/mcp/servers", Some(body))
            .await
            .map_err(|e| Self::err_context(e, "create mcp server"))?;
        serde_json::from_value(v).context("decode McpServer")
    }

    pub async fn connect_mcp_server(&self, id: &str) -> Result<Value> {
        self.request::<()>(Method::POST, &format!("/v1/mcp/servers/{}/connect", id), None)
            .await
            .map_err(|e| Self::err_context(e, "connect mcp server"))
    }

    pub async fn delete_mcp_server(&self, id: &str) -> Result<()> {
        self.request::<()>(Method::DELETE, &format!("/v1/mcp/servers/{}", id), None)
            .await
            .map_err(|e| Self::err_context(e, "delete mcp server"))?;
        Ok(())
    }

    // ---- Logs / analytics --------------------------------------------

    /// Fetch a page of audit log entries (#80). Filters compose with
    /// AND. Pass `None` to disable a filter. `since` / `until` are
    /// RFC 3339 timestamp strings.
    pub async fn get_audit(
        &self,
        action: Option<&str>,
        resource: Option<&str>,
        user_id: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Value> {
        let mut path = format!("/v1/audit?limit={}&offset={}", limit, offset);
        if let Some(a) = action {
            path.push_str(&format!("&action={}", urlencoding::encode(a)));
        }
        if let Some(r) = resource {
            path.push_str(&format!("&resource={}", urlencoding::encode(r)));
        }
        if let Some(u) = user_id {
            path.push_str(&format!("&user_id={}", urlencoding::encode(u)));
        }
        if let Some(s) = since {
            path.push_str(&format!("&since={}", urlencoding::encode(s)));
        }
        if let Some(u) = until {
            path.push_str(&format!("&until={}", urlencoding::encode(u)));
        }
        self.request::<()>(Method::GET, &path, None)
            .await
            .map_err(|e| Self::err_context(e, "get audit"))
    }

    pub async fn get_logs(&self, limit: u32) -> Result<Value> {
        let path = format!("/v1/user/logs?limit={}", limit);
        self.request::<()>(Method::GET, &path, None)
            .await
            .map_err(|e| Self::err_context(e, "get logs"))
    }

    pub async fn get_analytics(&self, range: &str) -> Result<Value> {
        let path = format!("/v1/user/analytics?range={}", range);
        self.request::<()>(Method::GET, &path, None)
            .await
            .map_err(|e| Self::err_context(e, "get analytics"))
    }

    // ---- Chat completions --------------------------------------------

    /// Non-streaming chat. For streaming, use [`Client::request`] with
    /// `stream: true` and read the response body as SSE bytes.
    pub async fn chat(&self, body: &ChatRequest) -> Result<Value> {
        self.request(Method::POST, "/v1/chat/completions", Some(body))
            .await
            .map_err(|e| Self::err_context(e, "chat completions"))
    }

    // ---- YAML config --------------------------------------------------

    /// Apply a YAML config blob (the same schema as `mawigateway.yaml`).
    /// Server-side this triggers an upsert on providers/models/services
    /// scoped to the calling user.
    pub async fn apply_config(&self, yaml_text: &str) -> Result<Value> {
        let url = format!("{}/v1/config/apply", self.base_url);
        let mut req = self.http.post(&url).header(
            header::CONTENT_TYPE,
            "application/x-yaml",
        );
        if let Some(key) = &self.api_key {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", key));
        }
        let resp = req.body(yaml_text.to_string()).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow!("apply_config: HTTP {}: {}", status, text));
        }
        serde_json::from_str(&text).context("decode apply response")
    }
}

// ---------------------------------------------------------------------------
// Typed payloads. Keep these aligned with the gateway's poem-openapi
// schemas in `backend/gateway/src/api.rs`. When you add or rename a
// field there, mirror it here — that's the only "API change" workflow.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMe {
    pub id: Uuid,
    pub email: Option<String>,
    pub display_name: Option<String>,
    #[serde(default)]
    pub tier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    /// Scope list. `["admin"]` = full access; subsets like `["read"]`
    /// or `["chat:my-service"]` indicate a least-privilege key. See
    /// the MawiGateway scope reference for the full set.
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedApiKey {
    pub id: Uuid,
    pub raw_key: String,
    pub prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    /// Echo of the scopes the new key was minted with. Defaults to
    /// `["admin"]` if the create call omitted scopes.
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: Uuid,
    pub name: String,
    pub provider_type: String,
    #[serde(default)]
    pub has_api_key: bool,
    #[serde(default)]
    pub api_endpoint: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProvider {
    pub name: String,
    pub provider_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: Uuid,
    pub name: String,
    pub provider: Uuid,
    pub modality: String,
    #[serde(default)]
    pub health_status: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub provider: Uuid,
    pub modality: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    #[serde(default)]
    pub service_type: Option<String>,
    #[serde(default)]
    pub strategy: Option<String>,
    #[serde(default)]
    pub modality: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub model_ids: Vec<Uuid>,
    /// Alternate names that route to this service (e.g.
    /// `["gpt-4o", "claude-3-5-sonnet"]`). Empty by default.
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateService {
    pub name: String,
    pub service_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub model_ids: Vec<Uuid>,
    /// Optional alternate names that should route to this service.
    /// Each must be unique across the service namespace; the gateway
    /// returns 409 if a name is already taken.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

/// PATCH-style update body for `PUT /v1/services/:name`. Every field is
/// optional — only the ones you set get applied; everything else is
/// left as-is. Pass `aliases: Some(vec![])` to clear the alias list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateService {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrails: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planner_model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: Uuid,
    pub name: String,
    pub server_type: String,
    pub image_or_command: String,
    pub status: String,
    #[serde(default)]
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMcpServer {
    pub name: String,
    pub server_type: String,
    pub image_or_command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_vars: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub service: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub stream: bool,
}
