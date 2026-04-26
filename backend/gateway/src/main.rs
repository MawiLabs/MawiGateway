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

    // Initialize PostgreSQL database
    let database_url = std::env::var("DATABASE_URL").expect("🔥 DATABASE_URL not set in .env. Please configure it to point to your persistent database.");

    tracing::info!("DATABASE_URL detected (value redacted)");
    let pool = mawi_core::db::init_db(&database_url).await?;

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

    // Create unified OpenAPI service for Swagger UI
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
        ),
        "MaWi API",
        "1.0",
    )
    .server("http://localhost:8030/v1");
    let ui = api_service.swagger_ui();
    let spec = api_service.spec();

    // CORS configuration. Fail-secure: if `CORS_ALLOWED_ORIGINS` is unset,
    // do NOT silently fall back to `http://localhost:3001` — a forgotten
    // env var in prod previously meant the gateway accepted credentialed
    // requests from a developer's laptop. Empty origins → browsers see a
    // CORS error (loud, visible failure) instead of an open door (#62).
    let cors_origins: Vec<String> = match std::env::var("CORS_ALLOWED_ORIGINS") {
        Ok(s) if !s.trim().is_empty() => {
            s.split(',').map(|s| s.trim().to_string()).collect()
        }
        _ => {
            tracing::error!(
                "CORS_ALLOWED_ORIGINS is not set — rejecting all browser requests. \
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
        .at("/health", get(health::health_check));

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

    // License Provider Injection
    #[cfg(feature = "enterprise")]
    let app = {
        let license_manager = std::sync::Arc::new(mawi_enterprise::license::LicenseManager::new());
        // Attempt to reload license on startup (fire and forget result, logs to stdout)
        let _ = license_manager.reload().await;
        app.with(poem::middleware::AddData::new(
            license_manager as std::sync::Arc<dyn mawi_core::license::LicenseProvider>,
        ))
    };

    #[cfg(not(feature = "enterprise"))]
    let app = app.with(poem::middleware::AddData::new(std::sync::Arc::new(
        mawi_core::license::OssLicenseProvider,
    )
        as std::sync::Arc<dyn mawi_core::license::LicenseProvider>));

    tracing::info!(
        port = 8030,
        swagger = "/swagger-ui",
        "MaWi Gateway starting"
    );

    Server::new(TcpListener::bind("0.0.0.0:8030"))
        .run_with_graceful_shutdown(
            app,
            async move {
                let _ = tokio::signal::ctrl_c().await;
                tracing::warn!("shutdown signal received, draining in-flight requests");
            },
            Some(std::time::Duration::from_secs(10)),
        )
        .await?;

    Ok(())
}
