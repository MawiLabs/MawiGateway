//! `print_openapi` — dump the gateway's OpenAPI spec to stdout.
//!
//! Build the same `OpenApiService` that `main.rs` mounts, but skip the
//! HTTP server: just print the JSON spec and exit. Run as part of CI
//! after every API change so `openapi.json` in the repo always reflects
//! the live surface, and downstream tools (Postman, openapi-typescript,
//! openapi-generator) can consume it offline.
//!
//! Usage:
//! ```bash
//! cargo run --bin print_openapi --quiet > openapi.json
//! ```
//!
//! Why not a `--print-openapi` flag on the main binary? The main binary
//! also runs migrations and seeds — irrelevant for spec generation, and
//! they require a live DB. This tool uses a lazy pool that never
//! actually connects, so it works in any CI environment.

use gateway::analytics::AnalyticsApi;
use gateway::api::ModelsApi;
use gateway::audit_api::AuditApi;
use gateway::auth_api::AuthApi;
use gateway::chat_new::ChatApi;
use gateway::executor::Executor;
use gateway::mcp_api::McpApi;
use gateway::mcp_client::McpManager;
use gateway::organizations::OrganizationsApi;
use gateway::topology::TopologyApi;
use gateway::user_api::UserApi;
use poem_openapi::OpenApiService;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `connect_lazy` defers the actual TCP connection until the first
    // query. We never query, so this URL is purely a placeholder.
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://nobody@127.0.0.1:5432/none")
        .expect("pool builder should accept any well-formed URL");

    let mcp_manager = Arc::new(RwLock::new(McpManager::new()));
    let executor = Arc::new(Executor::new(pool.clone(), mcp_manager.clone()));

    let api_service = OpenApiService::new(
        (
            ModelsApi { pool: pool.clone() },
            TopologyApi { pool: pool.clone() },
            AnalyticsApi { pool: pool.clone() },
            AuthApi { pool: pool.clone() },
            UserApi { pool: pool.clone() },
            OrganizationsApi { pool: pool.clone() },
            ChatApi {
                executor: executor.clone(),
            },
            McpApi::new(pool.clone(), mcp_manager.clone()),
            AuditApi { pool: pool.clone() },
        ),
        "MaWi API",
        env!("CARGO_PKG_VERSION"),
    )
    .description(
        "Unified gateway API. Operational endpoints (/health, /metrics, /spec, /swagger-ui) \
         are mounted outside this OpenAPI surface.",
    )
    .server("http://localhost:8030/v1");

    println!("{}", api_service.spec());
    Ok(())
}
