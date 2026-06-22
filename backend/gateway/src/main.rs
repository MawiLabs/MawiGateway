use poem::middleware::Cors;
use poem::{get, listener::TcpListener, post, EndpointExt, Route, Server};
use poem_openapi::OpenApiService;

use gateway::analytics::AnalyticsApi;
use gateway::api::ModelsApi;
use gateway::audio;
use gateway::auth_api::AuthApi;
use gateway::chat_new::ChatApi;
use gateway::executor::Executor;
use gateway::health;
use gateway::images;
use gateway::mcp_api::McpApi;
use gateway::observability;
use gateway::organizations::OrganizationsApi;
use gateway::speech_to_speech;
use gateway::topology::TopologyApi;
use gateway::transcription;
use gateway::user_api::UserApi;
use gateway::video;
use mawi_core::auth::middleware::AuthMiddleware;

use std::sync::Arc;

/// Metrics endpoint handler
fn metrics_endpoint(_req: poem::Request) -> String {
    gateway::metrics::gather_metrics()
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load .env file
    dotenv::dotenv().ok();

    // Initialize structured tracing (LOG_FORMAT=json for production)
    observability::init_tracing();

    // Initialize PostgreSQL database. Closes #67: missing MG_DATABASE_URL was
    // a hard `expect()` panic with a backtrace; that's not a useful signal
    // for an operator who just hasn't filled out their env. Now we return
    // a clean `anyhow::Error` from main, which exits 1 with a one-line
    // message and no stack trace.
    let database_url = std::env::var("MG_DATABASE_URL").map_err(|_| {
        anyhow::anyhow!(
            "MG_DATABASE_URL is not set. Point it at your Postgres instance, e.g. \
             postgres://mawi:password@localhost:5432/mawi"
        )
    })?;

    tracing::info!("DATABASE_URL detected (value redacted)");
    let pool = mawi_core::db::init_db(&database_url).await?;

    // Re-encrypt any plaintext API keys left over from before the #32 fix.
    // After this returns, no provider/model row has a plaintext api_key,
    // and decrypt_key() can refuse plaintext as its default. We log the
    // outcome but don't abort boot: a partial migration is no worse than
    // today's status quo, and runtime errors will surface any stragglers.
    match mawi_core::security::migrate_plaintext_keys(&pool).await {
        Ok(0) => tracing::debug!("no plaintext API keys to migrate"),
        Ok(n) => tracing::info!(rotated = n, "re-encrypted plaintext API keys at boot"),
        Err(e) => tracing::error!(
            error = %e,
            "plaintext-key migration failed — some rows may still be unprotected"
        ),
    }

    // Apply mawigateway.yaml if MG_CONFIG_FILE is set. Idempotent
    // upsert — re-running with the same file is a no-op. Failures
    // here abort boot so an operator can't silently drift away from
    // the file they thought was authoritative.
    gateway::config_loader::apply_config_file_if_present(&pool).await?;

    // Start the idempotency-cache cleanup sweeper (#41). Background task,
    // wakes every IDEMPOTENCY_CLEANUP_INTERVAL_SECS (default 1 hour) to
    // delete rows past their TTL. Runs for the lifetime of the process.
    gateway::idempotency::start_cleanup_task(pool.clone());

    // Semantic cache purger (Tier-2 #6). Same pattern as idempotency:
    // background tokio task, wakes hourly to delete expired rows.
    // Idempotent across replicas — safe to run on every pod.
    gateway::semantic_cache::spawn_purger(pool.clone(), std::time::Duration::from_secs(3600));

    // Shared MCP Manager (must be same instance for API and Executor)
    let mcp_manager = std::sync::Arc::new(tokio::sync::RwLock::new(
        gateway::mcp_client::McpManager::new(),
    ));

    // Auto-connect MCP servers from database (load ALL servers, preserve configs)
    {
        tracing::info!("loading MCP servers from database");
        let servers = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, name, server_type, image_or_command, args, env_vars FROM mcp_servers",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        let manager = mcp_manager.write().await;
        for (id, name, server_type, command, args_str, env_vars_str) in servers {
            tracing::info!(server_id = %id, name = %name, "reconnecting MCP server");
            let env_map: std::collections::HashMap<String, String> =
                serde_json::from_str(&env_vars_str).unwrap_or_default();
            let args_list: Vec<String> = serde_json::from_str(&args_str).unwrap_or_default();

            let server_type_enum = match server_type.as_str() {
                "docker" => gateway::mcp_client::ServerType::Docker,
                _ => gateway::mcp_client::ServerType::Stdio,
            };

            let config = gateway::mcp_client::McpServerConfig {
                id: id.clone(),
                name: name.clone(),
                server_type: server_type_enum,
                image_or_command: command,
                args: args_list,
                env_vars: env_map,
            };

            match manager.connect(&config).await {
                Ok(_) => {
                    tracing::info!(server_id = %id, name = %name, "MCP server reconnected");
                    let _ =
                        sqlx::query("UPDATE mcp_servers SET status = 'connected' WHERE id = $1")
                            .bind(&id)
                            .execute(&pool)
                            .await;
                }
                Err(e) => {
                    tracing::warn!(
                        server_id = %id,
                        name = %name,
                        error = %e,
                        "could not reconnect MCP server (config preserved)"
                    );
                    let _ =
                        sqlx::query("UPDATE mcp_servers SET status = 'disconnected' WHERE id = $1")
                            .bind(&id)
                            .execute(&pool)
                            .await;
                }
            }
        }
    }

    // Create executor with real provider integration
    let executor = Arc::new(Executor::new(pool.clone(), mcp_manager.clone()));

    // Create unified OpenAPI service for Swagger UI.
    //
    // Operational endpoints (`/health`, `/metrics`, `/spec`, `/swagger-ui`)
    // are intentionally NOT in the OpenAPI surface: they're for
    // load-balancers, scrapers, and humans, not API callers, and
    // advertising them widens the attack surface (e.g. `/metrics`
    // discoverability for unauthenticated scraping). The description
    // string below points operators at them so they're not invisible —
    // see #59.
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
            gateway::audit_api::AuditApi { pool: pool.clone() },
        ),
        "MaWi API",
        "1.0",
    )
    .description(
        "Unified gateway API. Operational endpoints — `/health` (liveness), \
         `/metrics` (Prometheus, on by default; opt out with \
         `DISABLE_METRICS=true`), `/spec` (raw OpenAPI JSON), and \
         `/swagger-ui` — are mounted outside this OpenAPI surface and \
         not listed below.",
    )
    .server("http://localhost:8030/v1");
    let ui = api_service.swagger_ui();
    let spec = api_service.spec();

    // CORS configuration. Fail-secure: if `MG_CORS_ALLOWED_ORIGINS` is unset,
    // do NOT silently fall back to `http://localhost:3001` — a forgotten
    // env var in prod previously meant the gateway accepted credentialed
    // requests from a developer's laptop. Empty origins → browsers see a
    // CORS error (loud, visible failure) instead of an open door (#62).
    let cors_origins: Vec<String> = match std::env::var("MG_CORS_ALLOWED_ORIGINS") {
        Ok(s) if !s.trim().is_empty() => s.split(',').map(|s| s.trim().to_string()).collect(),
        _ => {
            tracing::error!(
                "MG_CORS_ALLOWED_ORIGINS is not set — rejecting all browser requests. \
                 Set it to a comma-separated list of trusted origins (e.g. https://app.example.com)."
            );
            Vec::new()
        }
    };

    tracing::info!(origins = ?cors_origins, "CORS configured");

    let cors = Cors::new()
        .allow_origins(cors_origins)
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization", "Cookie"])
        .allow_credentials(true);

    // Protected Routes (require auth)
    let protected_routes = Route::new()
        .nest("/v1", api_service) // Combined Models, Providers, Services, Topology, Analytics, Auth, User, Chat
        // Image generation endpoint
        .at(
            "/v1/images/generations",
            post(images::image_generations).data(executor.clone()),
        )
        // Text-to-speech endpoint
        .at(
            "/v1/audio/speech",
            post(audio::text_to_speech).data(executor.clone()),
        )
        // Speech-to-text endpoint
        .at(
            "/v1/audio/transcriptions",
            post(transcription::transcribe_audio).data(executor.clone()),
        )
        // Speech-to-speech endpoint
        .at(
            "/v1/audio/speech-to-speech",
            post(speech_to_speech::speech_to_speech_endpoint).data(executor.clone()),
        )
        // Video generation endpoint
        .at(
            "/v1/videos/generations",
            post(video::generate_video).data(executor.clone()),
        )
        // Video job status polling
        .at(
            "/v1/videos/jobs/:job_id/:model_id",
            get(video::poll_video_job).data(executor.clone()),
        )
        // Video content proxy
        .at(
            "/v1/videos/content/:generation_id/:model_id",
            get(video::proxy_video_content).data(executor.clone()),
        )
        .with(AuthMiddleware);

    // Build routes
    let mut app = Route::new()
        .nest("/", protected_routes)
        .nest("/swagger-ui", ui)
        .at("/spec", poem::endpoint::make_sync(move |_| spec.clone()))
        .at("/health", get(health::health_check))
        // /v1/version returns build_sha + version + build_time (#66).
        // Outside the OpenAPI surface for the same reason as /health
        // and /metrics — operational endpoint, not an API call.
        .at("/v1/version", get(health::version_info));

    // Metrics endpoint — on by default. Opt out with DISABLE_METRICS=true.
    if gateway::metrics::metrics_enabled() {
        app = app.at("/metrics", poem::endpoint::make_sync(metrics_endpoint));
        tracing::info!("metrics endpoint mounted at /metrics");
    } else {
        tracing::info!("metrics endpoint disabled via DISABLE_METRICS=true");
    }

    let app = app
        .with(poem::middleware::AddData::new(pool))
        .with(cors)
        // RequestContext must be the OUTERMOST middleware so its span
        // wraps every handler — including auth + downstream provider calls.
        .with(observability::RequestContext);

    // License provider — always the OSS implementation. The enterprise
    // crate isn't part of this workspace, so the previous cfg-gated
    // alternative would only have failed any build that actually
    // enabled the `enterprise` feature. When the enterprise crate
    // returns, gate this with `#[cfg(feature = "enterprise")]` then.
    let app = app.with(poem::middleware::AddData::new(std::sync::Arc::new(
        mawi_core::license::OssLicenseProvider,
    )
        as std::sync::Arc<dyn mawi_core::license::LicenseProvider>));

    // Graceful-shutdown grace window. Video generation (Sora, Veo) can
    // take 60s+; the previous 10s default cut those calls off mid-flight,
    // wasting provider tokens and surfacing as 5xx to the caller. Default
    // is now 60s (covers most providers' p99) and is overridable so SREs
    // can match their orchestrator's terminationGracePeriodSeconds. See #53.
    let shutdown_grace_secs: u64 = std::env::var("MG_SHUTDOWN_GRACE_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    tracing::info!(
        port = 8030,
        swagger = "/swagger-ui",
        shutdown_grace_secs,
        "MaWi Gateway starting"
    );

    Server::new(TcpListener::bind("0.0.0.0:8030"))
        .run_with_graceful_shutdown(
            app,
            async move {
                let _ = tokio::signal::ctrl_c().await;
                tracing::warn!(
                    shutdown_grace_secs,
                    "shutdown signal received, draining in-flight requests"
                );
            },
            Some(std::time::Duration::from_secs(shutdown_grace_secs)),
        )
        .await?;

    Ok(())
}
