//! # `mg-mcp` — MawiGateway as an MCP server
//!
//! This binary speaks the **Model Context Protocol** (JSON-RPC 2.0 over
//! stdio) so AI agents like Claude Code, Cursor and Cline can use the
//! gateway as a tool. Drop it in your agent's MCP config:
//!
//! ```jsonc
//! // ~/.config/claude/mcp.json
//! {
//!   "mcpServers": {
//!     "mg": {
//!       "command": "mg-mcp",
//!       "env": { "MG_API_KEY": "sk_live_..." }
//!     }
//!   }
//! }
//! ```
//!
//! The agent can then call `mg_chat`, `mg_list_services`,
//! `mg_list_models`, etc. without ever talking to MawiGateway's HTTP
//! API directly.
//!
//! ## Tools exposed
//!
//! Read tools (always available):
//! - `mg_whoami` — current user
//! - `mg_list_providers`
//! - `mg_list_models`
//! - `mg_list_services`
//! - `mg_list_mcp_servers`
//! - `mg_get_logs`
//! - `mg_get_analytics`
//!
//! Inference:
//! - `mg_chat` — call any service (the gateway handles routing)
//!
//! Write tools (gated behind `--allow-writes` flag, default off):
//! - `mg_create_service`
//! - `mg_register_provider`
//! - `mg_create_model`
//! - `mg_create_api_key`
//!
//! ## Why JSON-RPC over stdio (and not SSE/HTTP)?
//!
//! Every major MCP host today (Claude Code, Cursor, Cline,
//! mcp-inspector) supports stdio transport. SSE/HTTP is in the spec
//! but has fragmented adoption. Starting with stdio means we work
//! everywhere day-one. Adding SSE later is straightforward — the tool
//! definitions stay identical, only the framing changes.

use anyhow::Result;
use clap::Parser;
use mawi_client::{
    Client, CreateModel, CreateProvider, CreateService, ChatMessage, ChatRequest,
    UpdateService,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info, warn};

// ---------------------------------------------------------------------------
// Cli args. Read at startup; no runtime reconfig.
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(name = "mg-mcp", version, about = "MawiGateway as an MCP server.")]
struct Args {
    /// Gateway base URL.
    #[arg(long, env = "MG_GATEWAY_URL", default_value = "http://localhost:8030")]
    gateway_url: String,

    /// API key. Required for any tool to actually work, but the server
    /// still starts without one so MCP hosts can probe `tools/list`.
    #[arg(long, env = "MG_API_KEY")]
    api_key: Option<String>,

    /// Allow agents to call write tools (create_service, register_provider, …).
    /// Off by default so a misbehaving agent can't reconfigure your gateway.
    #[arg(long, env = "MG_MCP_ALLOW_WRITES")]
    allow_writes: bool,
}

// ---------------------------------------------------------------------------
// MCP protocol shapes. JSON-RPC 2.0 envelope plus the MCP method shapes
// we actually implement (initialize, tools/list, tools/call). The full
// spec is at https://spec.modelcontextprotocol.io — we implement the
// minimal subset required by every popular host.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default)]
    #[allow(dead_code)]
    jsonrpc: String,
    /// Notifications omit `id`; we still respond to method calls.
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

const PROTOCOL_VERSION: &str = "2024-11-05";

// ---------------------------------------------------------------------------
// Tool catalog. One entry per tool: name, description, JSON Schema for
// arguments. Hosts call `tools/list` to discover this catalog and feed
// the descriptions into their LLM prompts — so the descriptions ARE
// the contract: write them like the agent will be reading them.
// ---------------------------------------------------------------------------

fn tool_catalog(allow_writes: bool) -> Vec<Value> {
    let mut tools = vec![
        tool(
            "mg_whoami",
            "Return the currently authenticated MawiGateway user (id, email, tier). \
             Use this first to confirm credentials are working.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "mg_list_providers",
            "List configured AI providers (OpenAI, Anthropic, Azure, etc.) with their \
             types and whether an API key is attached. Use to discover what's available \
             before creating a service.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "mg_list_models",
            "List configured models with their modality (text/image/audio/...) and \
             current health status. Use to find a model id to attach to a service.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "mg_list_services",
            "List routing services. Each service is a named pool that accepts chat \
             requests and routes them to a model based on its strategy. The 'name' \
             field is what you pass to mg_chat.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "mg_list_mcp_servers",
            "List MCP servers the gateway is consuming as tool sources. Returns id, \
             name, server_type (docker/stdio/sse) and connection status.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool(
            "mg_get_logs",
            "Fetch recent request logs (default 50). Each entry includes timestamp, \
             service, model, status, latency_ms — useful for debugging why a chat \
             call returned an error.",
            json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 500,
                        "default": 50,
                        "description": "Number of log entries to return."
                    }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "mg_get_analytics",
            "Fetch usage analytics (request counts, costs, latency p50/p99) over a \
             time range. Use when an operator asks 'how is my gateway doing?'",
            json!({
                "type": "object",
                "properties": {
                    "range": {
                        "type": "string",
                        "enum": ["24h", "7d", "30d"],
                        "default": "24h",
                        "description": "Time window."
                    }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "mg_chat",
            "Send a chat completion through a MawiGateway service. The gateway picks \
             the underlying model based on the service's routing strategy (least_cost, \
             least_latency, planner, etc.). Returns the OpenAI-shaped completion. \
             Use this whenever the user asks to invoke an LLM through the gateway.",
            json!({
                "type": "object",
                "required": ["service", "messages"],
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Name of the service to route through (see mg_list_services)."
                    },
                    "messages": {
                        "type": "array",
                        "minItems": 1,
                        "description": "OpenAI-shape chat messages.",
                        "items": {
                            "type": "object",
                            "required": ["role", "content"],
                            "properties": {
                                "role": { "type": "string", "enum": ["system", "user", "assistant", "tool"] },
                                "content": { "type": "string" }
                            }
                        }
                    },
                    "max_tokens": { "type": "integer", "minimum": 1 },
                    "temperature": { "type": "number", "minimum": 0, "maximum": 2 }
                },
                "additionalProperties": false
            }),
        ),
    ];

    if allow_writes {
        tools.extend([
            tool(
                "mg_register_provider",
                "Register a new AI provider (OpenAI, Anthropic, Azure, …). The \
                 api_key is stored encrypted server-side. Requires --allow-writes.",
                json!({
                    "type": "object",
                    "required": ["name", "type"],
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "type": { "type": "string", "description": "openai | anthropic | azure | google | xai | mistral | elevenlabs | selfhosted" },
                        "api_key": { "type": "string" },
                        "endpoint": { "type": "string", "description": "Endpoint override (Azure/self-hosted)." },
                        "api_version": { "type": "string", "description": "API version (Azure)." }
                    },
                    "additionalProperties": false
                }),
            ),
            tool(
                "mg_create_model",
                "Create a model record under an existing provider. The 'modality' \
                 must match what the provider supports (text/image/audio/...). \
                 Requires --allow-writes.",
                json!({
                    "type": "object",
                    "required": ["name", "provider", "modality"],
                    "properties": {
                        "name": { "type": "string" },
                        "provider": { "type": "string", "format": "uuid", "description": "Provider id from mg_list_providers." },
                        "modality": { "type": "string", "enum": ["text", "image", "video", "audio", "speech-to-text", "speech-to-speech", "multimodal"] }
                    },
                    "additionalProperties": false
                }),
            ),
            tool(
                "mg_create_service",
                "Create a routing service. Pool services attach multiple models and \
                 route by strategy. Agentic services use a planner model + tools. \
                 Optional `aliases` lets OpenAI/Anthropic SDK users target the \
                 service via familiar model names (e.g. \"gpt-4o\") while still \
                 going through the gateway's routing. \
                 Requires --allow-writes.",
                json!({
                    "type": "object",
                    "required": ["name", "type"],
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "type": { "type": "string", "enum": ["POOL", "AGENTIC"] },
                        "strategy": { "type": "string", "enum": ["weighted_random", "least_cost", "least_latency", "health", "none", "planner"], "default": "weighted_random" },
                        "modality": { "type": "string", "default": "text" },
                        "description": { "type": "string" },
                        "model_ids": { "type": "array", "items": { "type": "string", "format": "uuid" } },
                        "aliases": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Alternate names that route to this service. Each must be unique across the namespace; the gateway returns 409 if a name is already taken by another service or alias."
                        }
                    },
                    "additionalProperties": false
                }),
            ),
            tool(
                "mg_create_api_key",
                "Generate a new API key for the calling user. The raw key is \
                 returned ONCE in this response — store it securely. \
                 Optional `scopes` field constrains what the key can do — \
                 prefer the narrowest scope that still works (e.g. `read` \
                 for monitoring, `chat:my-service` for an embedded \
                 single-service client). Requires --allow-writes.",
                json!({
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": { "type": "string", "minLength": 1, "description": "Friendly label, e.g. 'CI pipeline'." },
                        "scopes": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Scope list. Defaults to ['admin'] (full access) when omitted. Predefined: 'admin', 'read', 'chat', 'config:read', 'config:write'. Per-service: 'chat:<service-name>'. Multiple scopes ANDed by the gateway — pick the smallest set that satisfies the use case."
                        }
                    },
                    "additionalProperties": false
                }),
            ),
            tool(
                "mg_update_service",
                "Update an existing service. Every field is optional — only the \
                 ones you pass get applied. Pass `aliases: []` to clear the \
                 alias list, or omit the field to leave it unchanged. Useful \
                 for re-targeting a service to a different model pool, \
                 changing the strategy, or wiring up OpenAI-compat aliases \
                 without re-creating. Requires --allow-writes.",
                json!({
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": { "type": "string", "minLength": 1, "description": "The service to update." },
                        "type": { "type": "string", "enum": ["POOL", "AGENTIC"] },
                        "strategy": { "type": "string", "enum": ["weighted_random", "least_cost", "least_latency", "health", "none", "planner"] },
                        "description": { "type": "string" },
                        "planner_model_id": { "type": "string", "format": "uuid", "description": "For agentic services: which model is the planner." },
                        "system_prompt": { "type": "string" },
                        "max_iterations": { "type": "integer", "minimum": 1, "description": "Agentic loop iteration cap." },
                        "aliases": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Replace the alias list. Pass [] to clear; omit to leave unchanged."
                        }
                    },
                    "additionalProperties": false
                }),
            ),
        ]);
    }

    tools
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": schema,
    })
}

// ---------------------------------------------------------------------------
// Tool dispatch. One match arm per tool; arms call into the typed
// `mawi-client`. When the API surface evolves and `mawi-client` gains
// new typed methods, this is the only file to edit.
// ---------------------------------------------------------------------------

async fn dispatch_tool(
    client: &Client,
    name: &str,
    args: &Value,
    allow_writes: bool,
) -> anyhow::Result<Value> {
    match name {
        "mg_whoami" => {
            let me = client.whoami().await?;
            Ok(serde_json::to_value(me)?)
        }
        "mg_list_providers" => Ok(serde_json::to_value(client.list_providers().await?)?),
        "mg_list_models" => Ok(serde_json::to_value(client.list_models().await?)?),
        "mg_list_services" => Ok(serde_json::to_value(client.list_services().await?)?),
        "mg_list_mcp_servers" => Ok(serde_json::to_value(client.list_mcp_servers().await?)?),
        "mg_get_logs" => {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32)
                .unwrap_or(50);
            client.get_logs(limit).await
        }
        "mg_get_analytics" => {
            let range = args
                .get("range")
                .and_then(|v| v.as_str())
                .unwrap_or("24h");
            client.get_analytics(range).await
        }
        "mg_chat" => {
            let service = args
                .get("service")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'service'"))?
                .to_string();
            let messages: Vec<ChatMessage> = serde_json::from_value(
                args.get("messages")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("missing 'messages'"))?,
            )?;
            let req = ChatRequest {
                service,
                messages,
                max_tokens: args.get("max_tokens").and_then(|v| v.as_u64()).map(|n| n as u32),
                temperature: args
                    .get("temperature")
                    .and_then(|v| v.as_f64())
                    .map(|n| n as f32),
                stream: false,
            };
            client.chat(&req).await
        }

        // ---- write tools ------------------------------------------------
        "mg_register_provider" if allow_writes => {
            let body = CreateProvider {
                name: take_str(args, "name")?,
                provider_type: take_str(args, "type")?,
                api_key: opt_str(args, "api_key"),
                api_endpoint: opt_str(args, "endpoint"),
                api_version: opt_str(args, "api_version"),
            };
            let p = client.create_provider(&body).await?;
            Ok(serde_json::to_value(p)?)
        }
        "mg_create_model" if allow_writes => {
            let provider = take_str(args, "provider")?
                .parse()
                .map_err(|e| anyhow::anyhow!("provider is not a uuid: {}", e))?;
            let body = CreateModel {
                name: take_str(args, "name")?,
                provider,
                modality: take_str(args, "modality")?,
                api_endpoint: None,
                api_version: None,
                api_key: None,
            };
            let m = client.create_model(&body).await?;
            Ok(serde_json::to_value(m)?)
        }
        "mg_create_service" if allow_writes => {
            let model_ids: Vec<uuid::Uuid> = args
                .get("model_ids")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().and_then(|s| s.parse().ok()))
                        .collect()
                })
                .unwrap_or_default();
            let aliases: Vec<String> = args
                .get("aliases")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let body = CreateService {
                name: take_str(args, "name")?,
                service_type: take_str(args, "type")?,
                strategy: opt_str(args, "strategy"),
                modality: opt_str(args, "modality"),
                description: opt_str(args, "description"),
                model_ids,
                aliases,
            };
            let s = client.create_service(&body).await?;
            Ok(serde_json::to_value(s)?)
        }
        "mg_create_api_key" if allow_writes => {
            let name = take_str(args, "name")?;
            // `scopes` semantics: array (even empty) means "use this
            // list"; absence means "let the server default to admin."
            let scopes: Option<Vec<String>> = match args.get("scopes") {
                Some(Value::Array(a)) => Some(
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect(),
                ),
                Some(Value::Null) | None => None,
                _ => None,
            };
            let key = client.create_api_key(&name, scopes).await?;
            Ok(serde_json::to_value(key)?)
        }
        "mg_update_service" if allow_writes => {
            let name = take_str(args, "name")?;
            // `aliases` semantics: presence (even as []) means "replace
            // the list with this." Absence means "leave unchanged."
            // Mirror that: if the JSON key was set, build Some(...).
            let aliases: Option<Vec<String>> = match args.get("aliases") {
                Some(Value::Array(a)) => Some(
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect(),
                ),
                Some(Value::Null) | None => None,
                _ => None,
            };
            let body = UpdateService {
                service_type: opt_str(args, "type"),
                description: opt_str(args, "description"),
                strategy: opt_str(args, "strategy"),
                guardrails: None,
                pool_type: None,
                planner_model_id: opt_str(args, "planner_model_id"),
                system_prompt: opt_str(args, "system_prompt"),
                max_iterations: args
                    .get("max_iterations")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32),
                aliases,
            };
            let s = client.update_service(&name, &body).await?;
            Ok(serde_json::to_value(s)?)
        }

        // Block writes when not enabled, with a useful message so the agent learns.
        n if n.starts_with("mg_") && !allow_writes => Err(anyhow::anyhow!(
            "tool '{}' requires --allow-writes. Restart mg-mcp with the flag (or set \
             MG_MCP_ALLOW_WRITES=1) to enable write operations.",
            n
        )),

        _ => Err(anyhow::anyhow!("unknown tool: {}", name)),
    }
}

fn take_str(args: &Value, key: &str) -> anyhow::Result<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("missing required arg '{}'", key))
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// MCP `tools/call` returns content as an array of typed parts. We use a
// single text part with the JSON-encoded result. Hosts unmarshal the
// JSON inside their LLM prompt. Errors flow through the JSON-RPC error
// channel, not via `isError: true`, since the MCP spec is in flux on
// that point and most hosts handle JSON-RPC errors universally.
fn tool_call_result(payload: &Value) -> Value {
    json!({
        "content": [
            { "type": "text", "text": serde_json::to_string_pretty(payload).unwrap_or_default() }
        ]
    })
}

// ---------------------------------------------------------------------------
// JSON-RPC method dispatch.
// ---------------------------------------------------------------------------

async fn handle_request(
    client: &Client,
    allow_writes: bool,
    req: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let id = match req.id {
        Some(id) => id,
        None => {
            // Notification — MCP hosts send `notifications/initialized` etc.
            // We just absorb them.
            return None;
        }
    };

    let result: Result<Value, anyhow::Error> = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": { "listChanged": false }
            },
            "serverInfo": {
                "name": "mg-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => Ok(json!({ "tools": tool_catalog(allow_writes) })),
        "tools/call" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = req
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Null);
            match dispatch_tool(client, name, &args, allow_writes).await {
                Ok(payload) => Ok(tool_call_result(&payload)),
                Err(e) => Err(e),
            }
        }
        "ping" => Ok(json!({})),
        other => Err(anyhow::anyhow!("unknown method: {}", other)),
    };

    Some(match result {
        Ok(v) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(v),
            error: None,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32000,
                message: e.to_string(),
                data: None,
            }),
        },
    })
}

// ---------------------------------------------------------------------------
// stdio main loop. One newline-delimited JSON message per line.
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Logs go to stderr because stdout is the protocol channel. MCP
    // hosts surface stderr in their developer tools, so this is fine.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .compact()
        .init();

    if args.api_key.is_none() {
        warn!("starting without an API key. Set MG_API_KEY before any tool call will work.");
    }

    let client = Arc::new(Client::new(args.gateway_url.clone(), args.api_key.clone()));
    info!(
        "mg-mcp ready (gateway={}, writes={})",
        args.gateway_url, args.allow_writes
    );

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let stdout = tokio::io::stdout();
    let stdout = Arc::new(tokio::sync::Mutex::new(stdout));

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                error!("malformed JSON-RPC: {} (line: {:?})", e, line);
                continue;
            }
        };

        let client = client.clone();
        let stdout = stdout.clone();
        let allow_writes = args.allow_writes;
        // Spawn so a slow tool call doesn't block the read loop.
        tokio::spawn(async move {
            if let Some(resp) = handle_request(&client, allow_writes, req).await {
                let mut buf = match serde_json::to_vec(&resp) {
                    Ok(b) => b,
                    Err(e) => {
                        error!("serialize response: {}", e);
                        return;
                    }
                };
                buf.push(b'\n');
                let mut guard = stdout.lock().await;
                if let Err(e) = guard.write_all(&buf).await {
                    error!("stdout write: {}", e);
                }
                let _ = guard.flush().await;
            }
        });
    }

    Ok(())
}
